//! MemoryActor — single owner of SpectralDb.
//!
//! All graph access goes through actor messages. No Arc/Mutex sharing.
//! The actor IS the lock. SpectralDb operations are synchronous internally
//! (Mutex-based), but single-owner actor means no deadlock risk.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use spectral_db::crystallize::Crystal;
use spectral_db::{DbStatus, SpectralDb};

// ── Reply types ────────────────────────────────────────────────────────

/// A node recalled by spectral proximity.
#[derive(Debug, Clone)]
pub struct RecalledNode {
    pub oid: String,
    pub node_type: String,
    pub data: Vec<u8>,
    pub distance: f64,
}

// ── Messages ───────────────────────────────────────────────────────────

/// Messages the MemoryActor can receive.
///
/// Tuple variants — required by `ractor::call!` macro which constructs
/// a closure `|reply| Variant(args..., reply)`.
pub enum MemoryMsg {
    /// Store a node. Reply: Ok(oid) or Err(reason).
    Store(String, Vec<u8>, ractor::RpcReplyPort<Result<String, String>>),

    /// Recall nodes near a target OID within a spectral distance.
    Recall(String, f64, ractor::RpcReplyPort<Vec<RecalledNode>>),

    /// Crystallize settled subgraphs. Returns new crystals.
    Crystallize(ractor::RpcReplyPort<Vec<Crystal>>),

    /// Full database status snapshot.
    Status(ractor::RpcReplyPort<DbStatus>),

    /// Flush to git. Fire-and-forget.
    Flush,
}

// ── Actor state ────────────────────────────────────────────────────────

/// The actor's persistent state: owns the SpectralDb instance.
pub struct MemoryState {
    pub db: SpectralDb,
}

// ── Actor ──────────────────────────────────────────────────────────────

/// The MemoryActor: wraps SpectralDb in a Ractor actor.
///
/// Single ownership. No Arc. No Mutex sharing. The actor IS the lock.
pub struct MemoryActor;

/// Arguments to spawn a MemoryActor.
pub struct MemoryActorArgs {
    pub db: SpectralDb,
}

impl MemoryActor {
    /// Spawn a MemoryActor that owns the given SpectralDb.
    pub async fn spawn_with_db(
        name: Option<String>,
        db: SpectralDb,
    ) -> Result<ActorRef<MemoryMsg>, ractor::SpawnErr> {
        let (actor_ref, _handle) =
            Actor::spawn(name, MemoryActor, MemoryActorArgs { db }).await?;
        Ok(actor_ref)
    }
}

#[ractor::async_trait]
impl Actor for MemoryActor {
    type Msg = MemoryMsg;
    type State = MemoryState;
    type Arguments = MemoryActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: MemoryActorArgs,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(MemoryState { db: args.db })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        _message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        todo!("tick-1a: implement message handling")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA: &str = "grammar @memory {\n  type = node | edge\n}";

    fn open_test_db() -> (tempfile::TempDir, SpectralDb) {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let db = SpectralDb::open(dir.path(), SCHEMA, 1e-6, 5_000_000)
            .expect("failed to open SpectralDb");
        (dir, db)
    }

    #[tokio::test]
    async fn memory_actor_spawn_and_status() {
        let (_dir, db) = open_test_db();
        let actor_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn failed");

        let status = ractor::call!(actor_ref, MemoryMsg::Status)
            .expect("status call failed");

        assert_eq!(status.name, "memory");
        assert_eq!(status.node_count, 0);
        assert_eq!(status.edge_count, 0);
        assert_eq!(status.crystals, 0);

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn memory_actor_store_and_status() {
        let (_dir, db) = open_test_db();
        let actor_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn failed");

        // Store a node
        let result: Result<String, String> = ractor::call!(
            actor_ref,
            MemoryMsg::Store,
            "node".to_string(),
            b"hello".to_vec()
        )
        .expect("store call failed");
        assert!(result.is_ok(), "store should succeed");

        // Verify status reflects the insertion
        let status = ractor::call!(actor_ref, MemoryMsg::Status)
            .expect("status call failed");
        assert_eq!(status.node_count, 1);

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn memory_actor_store_invalid_type() {
        let (_dir, db) = open_test_db();
        let actor_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn failed");

        let result: Result<String, String> = ractor::call!(
            actor_ref,
            MemoryMsg::Store,
            "nonexistent".to_string(),
            b"data".to_vec()
        )
        .expect("store call failed");
        assert!(result.is_err(), "store with invalid type should fail");

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn memory_actor_crystallize_empty() {
        let (_dir, db) = open_test_db();
        let actor_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn failed");

        let crystals: Vec<Crystal> = ractor::call!(actor_ref, MemoryMsg::Crystallize)
            .expect("crystallize call failed");
        assert!(crystals.is_empty(), "no hot paths = no crystals");

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn memory_actor_flush() {
        let (_dir, db) = open_test_db();
        let actor_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn failed");

        // Store a node then flush — should not panic
        let _: Result<String, String> = ractor::call!(
            actor_ref,
            MemoryMsg::Store,
            "node".to_string(),
            b"persist-me".to_vec()
        )
        .expect("store call failed");

        // Flush is fire-and-forget — cast, not call
        actor_ref.cast(MemoryMsg::Flush).expect("flush cast failed");

        // Verify actor is still alive after flush
        let status = ractor::call!(actor_ref, MemoryMsg::Status)
            .expect("status after flush failed");
        assert_eq!(status.node_count, 1);

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn memory_actor_recall_empty() {
        let (_dir, db) = open_test_db();
        let actor_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn failed");

        let recalled: Vec<RecalledNode> = ractor::call!(
            actor_ref,
            MemoryMsg::Recall,
            "nonexistent".to_string(),
            1.0
        )
        .expect("recall call failed");
        assert!(recalled.is_empty(), "recall on empty db = empty");

        actor_ref.stop(None);
    }
}
