//! MemoryActor — single owner of SpectralDb.
//!
//! All graph access goes through actor messages. No Arc/Mutex sharing.
//! The actor IS the lock. SpectralDb operations are synchronous internally
//! (Mutex-based), but single-owner actor means no deadlock risk.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use serde::{Deserialize, Serialize};
use spectral_db::crystallize::Crystal;
use spectral_db::{DbStatus, SpectralDb};

use super::optics;

// ── Reply types ────────────────────────────────────────────────────────

/// A node recalled by spectral proximity.
#[derive(Debug, Clone)]
pub struct RecalledNode {
    pub oid: String,
    pub node_type: String,
    pub data: Vec<u8>,
    pub distance: f64,
}

/// Response from a pipeline query.
#[derive(Debug, Clone)]
pub struct QueryResponse {
    pub count: usize,
    pub loss: f64,
    pub nodes: Vec<QueryNode>,
}

/// A node in a query response (serializable for MCP).
#[derive(Debug, Clone)]
pub struct QueryNode {
    pub oid: String,
    pub node_type: String,
    pub data: String,
}

// ── Git-native optics reply types ─────────────────────────────────────

/// Diff between two commits, classified by spectral path prefix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    pub from: String,
    pub to: String,
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub changed_nodes: Vec<String>,
    pub added_edges: Vec<EdgeRef>,
    pub removed_edges: Vec<EdgeRef>,
    pub added_crystals: Vec<String>,
    pub removed_crystals: Vec<String>,
    pub metadata_changed: Vec<String>,
}

/// A from→to edge reference identified by node OIDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRef {
    pub from: String,
    pub to: String,
}

/// One commit on a node's blame chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameEntry {
    pub commit_oid: String,
    pub message: String,
    pub timestamp: i64,
    pub author: String,
    /// Fiedler eigenvalue of the graph at this commit, if the `profile` blob
    /// was present in the commit's tree (Phase 4+). `None` for older commits.
    pub fiedler_at_commit: Option<f64>,
}

/// One branch tip for `memory_branch` list mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchTip {
    pub name: String,
    pub commit_oid: String,
}

/// Result of `memory_branch` (either a created branch or a list).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BranchResult {
    Created { ref_name: String, commit_oid: String },
    List { branches: Vec<BranchTip> },
}

/// Result of `memory_checkout`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutResult {
    pub branch: String,
    pub commit_oid: String,
    /// Caller-visible note: in-memory state may be stale until restart.
    pub note: String,
}

/// One entry on a topic-note thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadEntry {
    pub commit_oid: String,
    pub timestamp: i64,
    pub body: String,
}

/// Result of `memory_cherrypick`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CherrypickResult {
    pub source_commit: String,
    pub new_head: String,
    /// Caller-visible note: in-memory state may be stale until restart.
    pub note: String,
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

    /// Execute a pipe-forward query pipeline. Reply: Ok((count, loss)) or Err(reason).
    Query(String, ractor::RpcReplyPort<Result<(usize, f64), String>>),

    /// Execute a pipe-forward query, return full results.
    QueryFull(String, ractor::RpcReplyPort<Result<QueryResponse, String>>),

    /// Run a spectral cascade cycle. Reply: true if any namespace changed.
    ///
    /// CascadeActor sends this instead of calling db.run_cascade() directly,
    /// ensuring cascade always runs on the authoritative in-memory db.
    RunCascade(ractor::RpcReplyPort<bool>),

    /// Store a node without waiting for OID reply. Used by inbox drain.
    StoreFireAndForget(String, Vec<u8>),

    /// Ingest all nodes of a given type: tokenize content, create inferred
    /// edges to token/compound nodes, discover coincidence edges across
    /// nodes that share tokens. Reply: Ok(edge_count) or Err(reason).
    IngestAll(String, ractor::RpcReplyPort<Result<usize, String>>),

    // ── Git-native optics ─────────────────────────────────────────────

    /// `memory_diff`: classify changes between two refs (defaults to HEAD~1..HEAD).
    Diff(
        Option<String>,
        Option<String>,
        ractor::RpcReplyPort<Result<DiffReport, String>>,
    ),

    /// `memory_blame`: return the commit chain that touched `nodes/{oid}/`.
    Blame(String, ractor::RpcReplyPort<Result<Vec<BlameEntry>, String>>),

    /// `memory_branch`: with `Some(name)` create branch at HEAD; with `None` list branches.
    Branch(
        Option<String>,
        ractor::RpcReplyPort<Result<BranchResult, String>>,
    ),

    /// `memory_checkout`: switch the active spectral branch (repoint symref HEAD).
    Checkout(String, ractor::RpcReplyPort<Result<CheckoutResult, String>>),

    /// `memory_thread`: walk all notes attached on `refs/spectral/notes/topics/{topic}`
    /// (or `refs/spectral/notes/{topic}` as fallback) chronologically.
    Thread(String, ractor::RpcReplyPort<Result<Vec<ThreadEntry>, String>>),

    /// `memory_cherrypick`: replay `commit_oid`'s tree changes onto current HEAD.
    Cherrypick(
        String,
        ractor::RpcReplyPort<Result<CherrypickResult, String>>,
    ),
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
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            MemoryMsg::Store(node_type, data, reply) => {
                let result = state
                    .db
                    .insert(&node_type, &data)
                    .map_err(|e| e.to_string());
                let _ = reply.send(result);
            }

            MemoryMsg::Recall(oid, distance, reply) => {
                let result_set = state.db.near(&oid, distance);
                let recalled: Vec<RecalledNode> = result_set
                    .nodes
                    .into_iter()
                    .map(|node| {
                        let dist = state
                            .db
                            .spectral_distance(&oid, &node.oid)
                            .unwrap_or(f64::INFINITY);
                        RecalledNode {
                            oid: node.oid,
                            node_type: node.node_type,
                            data: node.data,
                            distance: dist,
                        }
                    })
                    .collect();
                let _ = reply.send(recalled);
            }

            MemoryMsg::Crystallize(reply) => {
                let crystals = state.db.crystallize();
                let _ = reply.send(crystals);
            }

            MemoryMsg::Status(reply) => {
                let status = state.db.status();
                let _ = reply.send(status);
            }

            MemoryMsg::Flush => {
                if let Err(e) = state.db.flush() {
                    eprintln!("spectral memory: flush failed: {}", e);
                }
            }

            MemoryMsg::RunCascade(reply) => {
                let changed = state.db.run_cascade();
                let _ = reply.send(changed);
            }

            MemoryMsg::Query(query_str, reply) => {
                let result = state
                    .db
                    .query_pipeline(&query_str)
                    .map(|r| (r.count, r.loss))
                    .map_err(|e| e.to_string());
                let _ = reply.send(result);
            }

            MemoryMsg::StoreFireAndForget(node_type, data) => {
                let _ = state.db.insert(&node_type, &data);
            }

            MemoryMsg::IngestAll(node_type, reply) => {
                let result = state
                    .db
                    .ingest_all(&node_type)
                    .map(|results| {
                        results.iter().map(|r| r.coincidence_edges).sum()
                    })
                    .map_err(|e| e.to_string());
                let _ = reply.send(result);
            }

            MemoryMsg::Diff(from, to, reply) => {
                let result = optics::diff(state.db.repo_path(), from.as_deref(), to.as_deref());
                let _ = reply.send(result);
            }

            MemoryMsg::Blame(oid, reply) => {
                let result = optics::blame(state.db.repo_path(), &oid);
                let _ = reply.send(result);
            }

            MemoryMsg::Branch(name, reply) => {
                let result = optics::branch(state.db.repo_path(), name.as_deref());
                let _ = reply.send(result);
            }

            MemoryMsg::Checkout(name, reply) => {
                let result = optics::checkout(state.db.repo_path(), &name);
                let _ = reply.send(result);
            }

            MemoryMsg::Thread(topic, reply) => {
                let result = optics::thread(state.db.repo_path(), &topic);
                let _ = reply.send(result);
            }

            MemoryMsg::Cherrypick(commit_oid, reply) => {
                let result = optics::cherrypick(state.db.repo_path(), &commit_oid);
                let _ = reply.send(result);
            }

            MemoryMsg::QueryFull(query_str, reply) => {
                let result = state
                    .db
                    .query_pipeline(&query_str)
                    .map(|r| QueryResponse {
                        count: r.count,
                        loss: r.loss,
                        nodes: r
                            .nodes
                            .into_iter()
                            .map(|n| QueryNode {
                                oid: n.oid,
                                node_type: n.node_type,
                                data: String::from_utf8_lossy(&n.data).to_string(),
                            })
                            .collect(),
                    })
                    .map_err(|e| e.to_string());
                let _ = reply.send(result);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA: &str = "grammar @memory {\n  type = node | edge | eigenboard\n}";

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

    #[tokio::test]
    async fn memory_actor_query_pipeline() {
        let (_dir, db) = open_test_db();

        // Seed data
        db.insert("node", b"name=alice role=admin").unwrap();
        db.insert("node", b"name=bob role=user").unwrap();
        db.insert("node", b"name=carol role=admin").unwrap();

        let actor_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn failed");

        let result: Result<(usize, f64), String> = ractor::call!(
            actor_ref,
            MemoryMsg::Query,
            "find node |> where role = admin |> count".to_string()
        )
        .expect("query call failed");

        let (count, _loss) = result.unwrap();
        assert_eq!(count, 2, "alice and carol are admins");

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn memory_actor_query_returns_nodes() {
        let (_dir, db) = open_test_db();

        db.insert("eigenboard", b"repo:/x fiedler=0.08 nodes=50").unwrap();
        db.insert("eigenboard", b"repo:/y fiedler=0.02 nodes=30").unwrap();

        let actor_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn failed");

        let result: Result<QueryResponse, String> = ractor::call!(
            actor_ref,
            MemoryMsg::QueryFull,
            "find eigenboard |> where fiedler > 0.04".to_string()
        )
        .expect("query call failed");

        let response = result.unwrap();
        assert_eq!(response.count, 1);
        assert_eq!(response.nodes.len(), 1);
        assert!(response.nodes[0].data.contains("repo:/x"));

        actor_ref.stop(None);
    }

    // ── Ingest tests ───────��──────────────────────────────────────────

    const INGEST_SCHEMA: &str =
        "grammar @memory {\n  type = node | edge | eigenboard | observation | token | compound\n}";

    fn open_ingest_db() -> (tempfile::TempDir, SpectralDb) {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let db = SpectralDb::open(dir.path(), INGEST_SCHEMA, 1e-6, 5_000_000)
            .expect("failed to open SpectralDb");
        (dir, db)
    }

    #[tokio::test]
    async fn ingest_all_creates_coincidence_edges() {
        let (_dir, db) = open_ingest_db();

        // Store observations with overlapping content
        db.insert("observation", b"eigenvalue computation is fast").unwrap();
        db.insert("observation", b"eigenvalue decomposition is slow").unwrap();
        db.insert("observation", b"coffee breakfast sunshine").unwrap();

        let actor_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn failed");

        // IngestAll should tokenize observations and create coincidence edges
        let result: Result<usize, String> = ractor::call!(
            actor_ref,
            MemoryMsg::IngestAll,
            "observation".to_string()
        )
        .expect("ingest_all call failed");

        let coincidence_count = result.expect("ingest_all should succeed");
        assert!(
            coincidence_count >= 1,
            "overlapping observations should create coincidence edges, got {}",
            coincidence_count
        );

        // Verify edges exist in status
        let status = ractor::call!(actor_ref, MemoryMsg::Status)
            .expect("status call failed");
        assert!(
            status.edge_count > 0,
            "graph should have edges after ingest, got {}",
            status.edge_count
        );

        actor_ref.stop(None);
    }

    #[tokio::test]
    async fn ingest_all_edges_survive_flush_and_reopen() {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let db_path = dir.path().to_path_buf();

        let edge_count_before;
        {
            let db = SpectralDb::open(&db_path, INGEST_SCHEMA, 1e-6, 5_000_000)
                .expect("failed to open SpectralDb");

            db.insert("observation", b"spectral eigenvalue convergence").unwrap();
            db.insert("observation", b"spectral hash eigenvalue detection").unwrap();

            let actor_ref = MemoryActor::spawn_with_db(None, db)
                .await
                .expect("spawn failed");

            // Ingest
            let _: Result<usize, String> = ractor::call!(
                actor_ref,
                MemoryMsg::IngestAll,
                "observation".to_string()
            )
            .expect("ingest_all call failed");

            // Flush
            actor_ref.cast(MemoryMsg::Flush).expect("flush cast failed");

            // Sync: wait for flush to complete via a synchronous status call
            let status = ractor::call!(actor_ref, MemoryMsg::Status)
                .expect("status call failed");
            edge_count_before = status.edge_count;
            assert!(
                edge_count_before > 0,
                "must have edges before reopen, got 0"
            );

            actor_ref.stop(None);
        }

        // Reopen at the same path — edges must survive
        {
            let db = SpectralDb::open(&db_path, INGEST_SCHEMA, 1e-6, 5_000_000)
                .expect("failed to reopen SpectralDb");
            let (_, edge_count_after) = db.graph_stats();
            assert!(
                edge_count_after > 0,
                "edges must survive reopen, got 0 (was {} before)",
                edge_count_before
            );
        }
    }
}
