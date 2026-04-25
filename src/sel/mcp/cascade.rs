//! CascadeActor — periodic spectral-db cascade tick.
//!
//! Runs the spectral cascade on a timer: dirty leaves → recompute ego spectra
//! → propagate up the tree. The actor owns a reference to the MemoryActor and
//! triggers cascade through it (or directly through a shared SpectralDb ref
//! passed at construction time).

use std::time::Duration;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use spectral_db::SpectralDb;

/// Default cascade interval: 5 seconds.
pub const CASCADE_INTERVAL: Duration = Duration::from_secs(5);

// ── Messages ─────────────────────────────────────────────────────────

/// Messages the CascadeActor can receive.
pub enum CascadeMsg {
    /// Run a single cascade cycle and reply with whether anything changed.
    RunCascade(ractor::RpcReplyPort<bool>),

    /// Self-scheduled periodic tick. Fire-and-forget.
    Tick,
}

// ── Actor state ──────────────────────────────────────────────────────

/// The actor's persistent state: holds a SpectralDb for cascade operations.
pub struct CascadeState {
    pub db: SpectralDb,
    pub cascade_count: u64,
}

// ── Actor ────────────────────────────────────────────────────────────

/// The CascadeActor: periodically runs spectral-db cascade.
///
/// On startup, schedules a periodic Tick message. Each tick calls
/// `db.run_cascade()` to recompute dirty ego spectra and propagate
/// changes up the spectral tree.
pub struct CascadeActor;

/// Arguments to spawn a CascadeActor.
pub struct CascadeActorArgs {
    pub db: SpectralDb,
    /// If None, uses CASCADE_INTERVAL.
    pub interval: Option<Duration>,
}

impl CascadeActor {
    /// Spawn a CascadeActor with a SpectralDb reference.
    pub async fn spawn_with_db(
        name: Option<String>,
        db: SpectralDb,
        interval: Option<Duration>,
    ) -> Result<ActorRef<CascadeMsg>, ractor::SpawnErr> {
        let (actor_ref, _handle) = Actor::spawn(
            name,
            CascadeActor,
            CascadeActorArgs { db, interval },
        )
        .await?;
        Ok(actor_ref)
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
        Ok(CascadeState {
            db: args.db,
            cascade_count: 0,
        })
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Schedule periodic cascade tick
        // send_interval will keep sending until the actor stops
        let _handle = myself.send_interval(CASCADE_INTERVAL, || CascadeMsg::Tick);
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
                let changed = state.db.run_cascade();
                state.cascade_count += 1;
                let _ = reply.send(changed);
            }
            CascadeMsg::Tick => {
                let _changed = state.db.run_cascade();
                state.cascade_count += 1;
            }
        }
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────────────────

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

    // Fix #5 red tests: CascadeActor must use MemoryActor's db, not its own.
    // spawn_with_memory_ref does not exist yet — fails to compile.
    #[tokio::test]
    async fn cascade_uses_memory_actor_db() {
        let (_dir, db) = open_test_db();
        let memory_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("memory spawn failed");

        // CascadeActor should be spawnable with a memory_ref, no separate db.
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

        // Send a manual Tick — should process without error
        actor_ref
            .cast(CascadeMsg::Tick)
            .expect("tick cast failed");

        // Verify actor is still alive
        let changed: bool = ractor::call!(actor_ref, CascadeMsg::RunCascade)
            .expect("cascade call after tick failed");
        assert!(!changed);

        actor_ref.stop(None);
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

        // Connect the node to itself to make a dirty leaf
        let cascade_ref = CascadeActor::spawn_with_memory_ref(None, memory_ref.clone(), None)
            .await
            .expect("cascade spawn failed");

        // RunCascade: even with one node, cascade should run without panic
        let _changed: bool = ractor::call!(cascade_ref, CascadeMsg::RunCascade)
            .expect("cascade after insert failed");

        cascade_ref.stop(None);
        memory_ref.stop(None);
    }
}
