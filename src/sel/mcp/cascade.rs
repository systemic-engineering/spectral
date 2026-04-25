//! CascadeActor -- periodic spectral-db cascade tick.
//!
//! Runs the spectral cascade on a timer: dirty leaves -> recompute ego spectra
//! -> propagate up the tree.
//!
//! The actor holds a reference to MemoryActor and triggers cascade through it
//! via `MemoryMsg::RunCascade`. This ensures cascade always operates on the
//! single authoritative in-memory SpectralDb owned by MemoryActor, rather than
//! a separate stale copy opened at the same path.

use std::path::PathBuf;
use std::time::Duration;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use spectral_db::SpectralDb;

use super::memory::MemoryMsg;

/// Default cascade interval: 5 seconds.
pub const CASCADE_INTERVAL: Duration = Duration::from_secs(5);

// -- Messages ------------------------------------------------------------------

/// Messages the CascadeActor can receive.
pub enum CascadeMsg {
    /// Run a single cascade cycle and reply with whether anything changed.
    RunCascade(ractor::RpcReplyPort<bool>),

    /// Periodic tick: drain inbox, run cascade, reply with cascade result.
    /// Supports both fire-and-forget (cast) and call-and-reply patterns.
    Tick(ractor::RpcReplyPort<bool>),
}

// -- Actor state ---------------------------------------------------------------

/// The actor's persistent state: reference to MemoryActor's db.
pub struct CascadeState {
    /// Reference to MemoryActor -- cascade operations go through it.
    pub memory_ref: ActorRef<MemoryMsg>,
    pub cascade_count: u64,
    /// Actual interval used for periodic ticks.
    pub interval: Duration,
    /// Path to the git/spectral directory containing the inbox.
    /// None in tests that don't exercise inbox drain.
    pub db_path: Option<PathBuf>,
}

// -- Actor ---------------------------------------------------------------------

/// The CascadeActor: periodically triggers spectral-db cascade via MemoryActor.
///
/// On startup, schedules a periodic Tick message. Each tick sends
/// `MemoryMsg::RunCascade` to MemoryActor, which runs the cascade on the
/// authoritative in-memory SpectralDb. This eliminates the dual-SpectralDb
/// problem where CascadeActor previously maintained a separate stale copy.
pub struct CascadeActor;

/// Arguments to spawn a CascadeActor.
pub struct CascadeActorArgs {
    /// Reference to MemoryActor that owns the authoritative SpectralDb.
    pub memory_ref: ActorRef<MemoryMsg>,
    /// If None, uses CASCADE_INTERVAL.
    pub interval: Option<Duration>,
    /// Path to the project root (parent of `.git/spectral/inbox/`).
    /// If None, inbox drain is skipped.
    pub db_path: Option<PathBuf>,
}

impl CascadeActor {
    /// Spawn a CascadeActor that routes cascade through the given MemoryActor.
    ///
    /// This is the primary constructor. CascadeActor does not own a SpectralDb;
    /// it delegates all cascade operations to MemoryActor via RunCascade messages.
    pub async fn spawn_with_memory_ref(
        name: Option<String>,
        memory_ref: ActorRef<MemoryMsg>,
        interval: Option<Duration>,
    ) -> Result<ActorRef<CascadeMsg>, ractor::SpawnErr> {
        let (actor_ref, _handle) = Actor::spawn(
            name,
            CascadeActor,
            CascadeActorArgs { memory_ref, interval, db_path: None },
        )
        .await?;
        Ok(actor_ref)
    }

    /// Compatibility shim: spawn with an owned SpectralDb.
    ///
    /// Wraps db in a MemoryActor, then spawns CascadeActor with that ref.
    /// Prefer `spawn_with_memory_ref` when a MemoryActor already exists.
    pub async fn spawn_with_db(
        name: Option<String>,
        db: SpectralDb,
        interval: Option<Duration>,
    ) -> Result<ActorRef<CascadeMsg>, ractor::SpawnErr> {
        let memory_ref = super::memory::MemoryActor::spawn_with_db(None, db).await?;
        Self::spawn_with_memory_ref(name, memory_ref, interval).await
    }
}

#[ractor::async_trait]
impl Actor for CascadeActor {
    type Msg = CascadeMsg;
    type State = CascadeState;
    type Arguments = CascadeActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: CascadeActorArgs,
    ) -> Result<Self::State, ActorProcessingErr> {
        let interval = args.interval.unwrap_or(CASCADE_INTERVAL);
        Ok(CascadeState {
            memory_ref: args.memory_ref,
            cascade_count: 0,
            interval,
            db_path: args.db_path,
        })
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Schedule periodic cascade tick using the configured interval.
        // Tick carries a reply port; for the periodic self-send we use cast via
        // a one-shot channel where we discard the reply.
        let interval = state.interval;
        let _handle = myself.send_interval(interval, || {
            let (tx, _rx) = ractor::concurrency::oneshot();
            CascadeMsg::Tick(tx.into())
        });
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            CascadeMsg::RunCascade(reply) => {
                // Drain inbox before cascade -- observations land in memory first.
                drain_inbox(state);
                // Delegate to MemoryActor -- single authoritative db.
                let changed = ractor::call!(state.memory_ref, MemoryMsg::RunCascade)
                    .unwrap_or(false);
                state.cascade_count += 1;
                // Flush after every cascade -- nodes must survive process death.
                let _ = state.memory_ref.cast(MemoryMsg::Flush);
                let _ = reply.send(changed);
            }
            CascadeMsg::Tick(reply) => {
                // Drain inbox before cascade -- observations land in memory first.
                drain_inbox(state);
                // Cascade tick through MemoryActor.
                let changed = ractor::call!(state.memory_ref, MemoryMsg::RunCascade)
                    .unwrap_or(false);
                state.cascade_count += 1;
                // Flush after every tick -- nodes must survive process death.
                let _ = state.memory_ref.cast(MemoryMsg::Flush);
                let _ = reply.send(changed);
            }
        }
        Ok(())
    }
}

// -- Inbox drain ---------------------------------------------------------------

/// Drain `.git/spectral/inbox/*.json` files into MemoryActor as observation nodes.
///
/// Called at the start of every Tick and RunCascade. Reads each JSON file,
/// formats it as a node content string, stores it via StoreFireAndForget, then
/// deletes the file. Errors (missing inbox dir, unreadable files) are silently
/// skipped — inbox drain is best-effort.
fn drain_inbox(state: &CascadeState) {
    let db_path = match &state.db_path {
        Some(p) => p,
        None => return,
    };
    let inbox = db_path.join("inbox");
    let entries = match std::fs::read_dir(&inbox) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(obs) = serde_json::from_str::<serde_json::Value>(&content) {
                let tool = obs["tool"].as_str().unwrap_or("unknown");
                let node_content = format!(
                    "tool:{} input:{} output:{}",
                    tool,
                    obs["input"].as_str().unwrap_or(""),
                    obs["output"].as_str().unwrap_or(""),
                );
                let _ = state.memory_ref.cast(super::memory::MemoryMsg::StoreFireAndForget(
                    "observation".to_string(),
                    node_content.into_bytes(),
                ));
            }
            let _ = std::fs::remove_file(&path);
        }
    }
}

// -- Tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sel::mcp::memory::{MemoryActor, MemoryActorArgs};

    const SCHEMA: &str = "grammar @memory {\n  type = node | edge\n}";

    fn open_test_db() -> (tempfile::TempDir, SpectralDb) {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let db = SpectralDb::open(dir.path(), SCHEMA, 1e-6, 5_000_000)
            .expect("failed to open SpectralDb");
        (dir, db)
    }

    // Fix #5: CascadeActor uses MemoryActor's db, not its own.
    #[tokio::test]
    async fn cascade_uses_memory_actor_db() {
        let (_dir, db) = open_test_db();
        let memory_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("memory spawn failed");

        // CascadeActor spawnable with memory_ref, no separate db.
        let cascade_ref = CascadeActor::spawn_with_memory_ref(None, memory_ref.clone(), None)
            .await
            .expect("cascade spawn failed");

        // RunCascade on empty db should return false
        let changed: bool = ractor::call!(cascade_ref, CascadeMsg::RunCascade)
            .expect("cascade call failed");
        assert!(!changed, "empty db has no dirty leaves");

        cascade_ref.stop(None);
        memory_ref.stop(None);
    }

    #[tokio::test]
    async fn cascade_actor_tick_does_not_panic() {
        let (_dir, db) = open_test_db();
        let memory_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("memory spawn failed");
        let actor_ref = CascadeActor::spawn_with_memory_ref(None, memory_ref.clone(), None)
            .await
            .expect("spawn failed");

        // Send a manual Tick -- should process without error
        let _: bool = ractor::call!(actor_ref, CascadeMsg::Tick)
            .expect("tick call failed");

        // Verify actor is still alive
        let changed: bool = ractor::call!(actor_ref, CascadeMsg::RunCascade)
            .expect("cascade call after tick failed");
        assert!(!changed);

        actor_ref.stop(None);
        memory_ref.stop(None);
    }

    // Flush must happen after cascade: nodes inserted via MemoryActor must be
    // persisted to git refs after a RunCascade call.
    #[tokio::test]
    async fn cascade_flushes_to_disk_after_run() {
        let (dir, db) = open_test_db();
        let memory_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("memory spawn failed");

        // Insert a node.
        let _: Result<String, String> = ractor::call!(
            memory_ref,
            crate::sel::mcp::memory::MemoryMsg::Store,
            "node".to_string(),
            b"flush-test".to_vec()
        )
        .expect("store failed");

        let cascade_ref = CascadeActor::spawn_with_memory_ref(None, memory_ref.clone(), None)
            .await
            .expect("cascade spawn failed");

        // RunCascade must also flush — git refs must appear on disk.
        let _changed: bool = ractor::call!(cascade_ref, CascadeMsg::RunCascade)
            .expect("RunCascade failed");

        // Drain MemoryActor mailbox (Status is synchronous — Flush precedes it).
        let _ = ractor::call!(memory_ref, crate::sel::mcp::memory::MemoryMsg::Status)
            .expect("status failed");

        // Verify flush wrote git refs (replaces manifest.json check).
        // Git stores refs as files under .git/refs/spectral/nodes/.
        let refs_dir = dir.path().join(".git/refs/spectral/nodes");
        let ref_count = if refs_dir.exists() {
            std::fs::read_dir(&refs_dir)
                .expect("read refs dir")
                .count()
        } else {
            // packed-refs: check edges.json as proxy for flush having run
            // (edges.json is still written by flush, refs may be packed)
            assert!(
                dir.path().join("edges.json").exists(),
                "edges.json must exist after RunCascade (flush proof)"
            );
            1 // edges.json proves flush ran
        };
        assert!(
            ref_count >= 1,
            "git refs must exist after RunCascade, got {}",
            ref_count
        );

        cascade_ref.stop(None);
        memory_ref.stop(None);
    }

    #[tokio::test]
    async fn cascade_drains_inbox_on_tick() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().to_path_buf();

        // Open a SpectralDb for the MemoryActor
        let obs_schema = "grammar @memory {\n  type = node | edge | eigenboard | observation\n}";
        let db = SpectralDb::open(&db_path, obs_schema, 1e-6, 5_000_000).expect("open");
        let memory_ref = MemoryActor::spawn_with_db(None, db).await.expect("spawn memory");

        // Create inbox with one observation
        let inbox = db_path.join("inbox");
        std::fs::create_dir_all(&inbox).unwrap();
        let obs = serde_json::json!({
            "tool": "Bash",
            "input": "ls",
            "output": "foo.rs",
            "timestamp": 1234567890u64,
        });
        std::fs::write(inbox.join("1.json"), serde_json::to_string(&obs).unwrap()).unwrap();

        // Spawn cascade with db_path
        let (cascade_ref, _) = ractor::Actor::spawn(
            None,
            CascadeActor,
            CascadeActorArgs {
                memory_ref: memory_ref.clone(),
                db_path: Some(db_path.clone()),
                interval: Some(std::time::Duration::from_secs(3600)),
            },
        )
        .await
        .expect("spawn cascade");

        // Trigger a tick
        let _: bool = ractor::call!(cascade_ref, CascadeMsg::Tick).expect("tick");

        // Inbox should be empty
        assert_eq!(
            std::fs::read_dir(&inbox).unwrap().count(),
            0,
            "inbox should be drained after tick"
        );

        // Node should be stored (give actor a moment to process the cast)
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let status: spectral_db::DbStatus =
            ractor::call!(memory_ref, MemoryMsg::Status).expect("status");
        assert!(
            status.node_count >= 1,
            "expected at least 1 node from inbox, got {}",
            status.node_count
        );

        cascade_ref.stop(None);
        memory_ref.stop(None);
    }

    // Change inserted via MemoryActor must be visible to CascadeActor's cascade.
    #[tokio::test]
    async fn cascade_sees_memory_actor_inserts() {
        let (_dir, db) = open_test_db();
        let memory_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("memory spawn failed");

        // Insert a node via MemoryActor
        let oid: Result<String, String> = ractor::call!(
            memory_ref,
            crate::sel::mcp::memory::MemoryMsg::Store,
            "node".to_string(),
            b"test".to_vec()
        )
        .expect("store call failed");
        assert!(oid.is_ok());

        let cascade_ref = CascadeActor::spawn_with_memory_ref(None, memory_ref.clone(), None)
            .await
            .expect("cascade spawn failed");

        // RunCascade: should run without panic
        let _changed: bool = ractor::call!(cascade_ref, CascadeMsg::RunCascade)
            .expect("cascade after insert failed");

        cascade_ref.stop(None);
        memory_ref.stop(None);
    }
}
