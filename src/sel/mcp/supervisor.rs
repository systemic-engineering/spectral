//! SpectralSupervisor — supervision tree for all MCP actors.
//!
//! Wraps MemoryActor, FateActor, LspActor, CompilerActor, CascadeActor,
//! and McpActor under a single supervisor. When a child crashes, the
//! supervisor receives a `SupervisionEvent` and restarts the failed actor.
//!
//! ## Restart policies
//!
//! - MemoryActor crash → restart (reopen SpectralDb)
//! - FateActor crash → restart with untrained weights
//! - LspActor crash → restart
//! - CompilerActor crash → restart
//! - CascadeActor crash → restart
//! - McpActor crash → stop supervisor (stdio connection is dead)

use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use ractor::{Actor, ActorProcessingErr, ActorRef, SupervisionEvent};
use spectral_db::SpectralDb;

use super::cascade::{CascadeActor, CascadeMsg};
use super::compiler::{CompilerActor, CompilerMsg};
use super::lsp::{LspActor, LspMsg};
use super::memory::{MemoryActor, MemoryMsg};
use super::server::{McpActor, McpMsg};
use crate::sel::fate_actor::{FateActor, FateMsg};

// ── Messages ─────────────────────────────────────────────────────────

/// Messages the SpectralSupervisor can receive.
pub enum SupervisorMsg {
    /// Query for the McpActor ref (used by the stdio loop).
    GetMcpRef(ractor::RpcReplyPort<ActorRef<McpMsg>>),

    /// Query for the LspActor ref (used by tower-lsp adapter).
    GetLspRef(ractor::RpcReplyPort<(ActorRef<LspMsg>, Arc<DashMap<String, String>>)>),

    /// Query for the CompilerActor ref.
    GetCompilerRef(ractor::RpcReplyPort<ActorRef<CompilerMsg>>),

    /// Query for the CascadeActor ref.
    GetCascadeRef(ractor::RpcReplyPort<ActorRef<CascadeMsg>>),
}

// ── Actor IDs ────────────────────────────────────────────────────────

/// Track which child actor corresponds to which role.
/// We use ractor ActorId (u64) to identify children in supervision events.
#[derive(Debug, Clone)]
pub struct ChildIds {
    pub memory_id: ractor::ActorId,
    pub fate_id: ractor::ActorId,
    pub lsp_id: ractor::ActorId,
    pub compiler_id: ractor::ActorId,
    pub cascade_id: ractor::ActorId,
    pub mcp_id: ractor::ActorId,
}

// ── Actor state ──────────────────────────────────────────────────────

/// The supervisor's persistent state: refs to all children + config for restarts.
pub struct SupervisorState {
    pub memory_ref: ActorRef<MemoryMsg>,
    pub fate_ref: ActorRef<FateMsg>,
    pub lsp_ref: ActorRef<LspMsg>,
    pub lsp_documents: Arc<DashMap<String, String>>,
    pub compiler_ref: ActorRef<CompilerMsg>,
    pub cascade_ref: ActorRef<CascadeMsg>,
    pub mcp_ref: ActorRef<McpMsg>,
    pub child_ids: ChildIds,
    /// Retained for restarts: path to .spectral/ directory.
    pub db_path: PathBuf,
    /// Schema used to open SpectralDb.
    pub db_schema: String,
    pub db_precision: f64,
    pub db_max_bytes: usize,
    /// Name prefix for child actors (None = unnamed).
    pub name_prefix: Option<String>,
}

// ── Actor ────────────────────────────────────────────────────────────

/// The SpectralSupervisor: parent of all MCP actors.
pub struct SpectralSupervisor;

/// Arguments to spawn the supervisor.
pub struct SupervisorArgs {
    pub db: SpectralDb,
    pub db_path: PathBuf,
    pub db_schema: String,
    pub db_precision: f64,
    pub db_max_bytes: usize,
    /// Optional prefix for child actor names. If None, children spawn unnamed
    /// (useful for tests to avoid ractor name registry collisions).
    pub name_prefix: Option<String>,
}

/// Derive a child name from the prefix and suffix.
fn child_name(prefix: &Option<String>, suffix: &str) -> Option<String> {
    prefix.as_ref().map(|p| format!("{}-{}", p, suffix))
}

impl SpectralSupervisor {
    /// Spawn the supervisor and all children.
    pub async fn spawn_all(
        db: SpectralDb,
        db_path: PathBuf,
        db_schema: String,
        db_precision: f64,
        db_max_bytes: usize,
    ) -> Result<ActorRef<SupervisorMsg>, ractor::SpawnErr> {
        let (actor_ref, _handle) = Actor::spawn(
            Some("spectral-supervisor".to_string()),
            SpectralSupervisor,
            SupervisorArgs {
                db,
                db_path,
                db_schema,
                db_precision,
                db_max_bytes,
                name_prefix: Some("spectral".to_string()),
            },
        )
        .await?;
        Ok(actor_ref)
    }
}

#[ractor::async_trait]
impl Actor for SpectralSupervisor {
    type Msg = SupervisorMsg;
    type State = SupervisorState;
    type Arguments = SupervisorArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: SupervisorArgs,
    ) -> Result<Self::State, ActorProcessingErr> {
        let supervisor_cell = myself.get_cell();

        // Spawn all children linked to this supervisor.
        // Using Actor::spawn_linked so supervision events flow to us.

        let prefix = &args.name_prefix;

        // 1. MemoryActor — owns the primary SpectralDb
        let (memory_ref, _) = Actor::spawn_linked(
            child_name(prefix, "memory"),
            MemoryActor,
            super::memory::MemoryActorArgs { db: args.db },
            supervisor_cell.clone(),
        )
        .await
        .map_err(|e| ActorProcessingErr::from(format!("failed to spawn MemoryActor: {}", e)))?;

        // 2. FateActor
        let (fate_ref, _) = Actor::spawn_linked(
            child_name(prefix, "fate"),
            FateActor,
            (),
            supervisor_cell.clone(),
        )
        .await
        .map_err(|e| ActorProcessingErr::from(format!("failed to spawn FateActor: {}", e)))?;

        // 3. LspActor
        let lsp_documents = Arc::new(DashMap::new());
        let (lsp_ref, _) = Actor::spawn_linked(
            child_name(prefix, "lsp"),
            LspActor,
            super::lsp::LspActorArgs {
                documents: Arc::clone(&lsp_documents),
            },
            supervisor_cell.clone(),
        )
        .await
        .map_err(|e| ActorProcessingErr::from(format!("failed to spawn LspActor: {}", e)))?;

        // 4. CompilerActor
        let (compiler_ref, _) = Actor::spawn_linked(
            child_name(prefix, "compiler"),
            CompilerActor,
            (),
            supervisor_cell.clone(),
        )
        .await
        .map_err(|e| ActorProcessingErr::from(format!("failed to spawn CompilerActor: {}", e)))?;

        // 5. CascadeActor — opens its own SpectralDb for cascade operations
        let cascade_db = reopen_db(&args.db_path, &args.db_schema, args.db_precision, args.db_max_bytes)
            .map_err(|e| ActorProcessingErr::from(format!("failed to open cascade db: {}", e)))?;
        let (cascade_ref, _) = Actor::spawn_linked(
            child_name(prefix, "cascade"),
            CascadeActor,
            super::cascade::CascadeActorArgs {
                db: cascade_db,
                interval: None,
            },
            supervisor_cell.clone(),
        )
        .await
        .map_err(|e| ActorProcessingErr::from(format!("failed to spawn CascadeActor: {}", e)))?;

        // 6. McpActor — depends on memory + fate refs
        let (mcp_ref, _) = Actor::spawn_linked(
            child_name(prefix, "mcp"),
            McpActor,
            super::server::McpActorArgs {
                memory: memory_ref.clone(),
                fate: fate_ref.clone(),
                lsp: Some(lsp_ref.clone()),
                project_path: Some(args.db_path.parent().unwrap_or(&args.db_path).to_path_buf()),
            },
            supervisor_cell,
        )
        .await
        .map_err(|e| ActorProcessingErr::from(format!("failed to spawn McpActor: {}", e)))?;

        let child_ids = ChildIds {
            memory_id: memory_ref.get_id(),
            fate_id: fate_ref.get_id(),
            lsp_id: lsp_ref.get_id(),
            compiler_id: compiler_ref.get_id(),
            cascade_id: cascade_ref.get_id(),
            mcp_id: mcp_ref.get_id(),
        };

        eprintln!("spectral supervisor: all children spawned");

        Ok(SupervisorState {
            memory_ref,
            fate_ref,
            lsp_ref,
            lsp_documents,
            compiler_ref,
            cascade_ref,
            mcp_ref,
            child_ids,
            db_path: args.db_path,
            db_schema: args.db_schema,
            db_precision: args.db_precision,
            db_max_bytes: args.db_max_bytes,
            name_prefix: args.name_prefix,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisorMsg::GetMcpRef(reply) => {
                let _ = reply.send(state.mcp_ref.clone());
            }
            SupervisorMsg::GetLspRef(reply) => {
                let _ = reply.send((state.lsp_ref.clone(), Arc::clone(&state.lsp_documents)));
            }
            SupervisorMsg::GetCompilerRef(reply) => {
                let _ = reply.send(state.compiler_ref.clone());
            }
            SupervisorMsg::GetCascadeRef(reply) => {
                let _ = reply.send(state.cascade_ref.clone());
            }
        }
        Ok(())
    }

    async fn handle_supervisor_evt(
        &self,
        myself: ActorRef<Self::Msg>,
        message: SupervisionEvent,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisionEvent::ActorFailed(dead_actor, panic_msg) => {
                let dead_id = dead_actor.get_id();
                let supervisor_cell = myself.get_cell();

                if dead_id == state.child_ids.mcp_id {
                    // McpActor crash → stdio connection is dead → stop everything
                    eprintln!(
                        "spectral supervisor: McpActor crashed ({}), stopping supervisor",
                        panic_msg
                    );
                    myself.stop(Some("McpActor crashed — stdio dead".to_string()));
                } else if dead_id == state.child_ids.memory_id {
                    eprintln!(
                        "spectral supervisor: MemoryActor crashed ({}), restarting",
                        panic_msg
                    );
                    match reopen_db(&state.db_path, &state.db_schema, state.db_precision, state.db_max_bytes) {
                        Ok(db) => {
                            match Actor::spawn_linked(
                                child_name(&state.name_prefix, "memory"),
                                MemoryActor,
                                super::memory::MemoryActorArgs { db },
                                supervisor_cell,
                            )
                            .await
                            {
                                Ok((new_ref, _)) => {
                                    state.child_ids.memory_id = new_ref.get_id();
                                    state.memory_ref = new_ref;
                                    eprintln!("spectral supervisor: MemoryActor restarted");
                                }
                                Err(e) => {
                                    eprintln!("spectral supervisor: failed to restart MemoryActor: {}", e);
                                    myself.stop(Some("MemoryActor restart failed".to_string()));
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("spectral supervisor: failed to reopen db: {}", e);
                            myself.stop(Some("MemoryActor restart failed (db)".to_string()));
                        }
                    }
                } else if dead_id == state.child_ids.fate_id {
                    eprintln!(
                        "spectral supervisor: FateActor crashed ({}), restarting with untrained weights",
                        panic_msg
                    );
                    match Actor::spawn_linked(
                        child_name(&state.name_prefix, "fate"),
                        FateActor,
                        (),
                        supervisor_cell,
                    )
                    .await
                    {
                        Ok((new_ref, _)) => {
                            state.child_ids.fate_id = new_ref.get_id();
                            state.fate_ref = new_ref;
                            eprintln!("spectral supervisor: FateActor restarted");
                        }
                        Err(e) => {
                            eprintln!("spectral supervisor: failed to restart FateActor: {}", e);
                        }
                    }
                } else if dead_id == state.child_ids.lsp_id {
                    eprintln!(
                        "spectral supervisor: LspActor crashed ({}), restarting",
                        panic_msg
                    );
                    let new_docs = Arc::new(DashMap::new());
                    match Actor::spawn_linked(
                        child_name(&state.name_prefix, "lsp"),
                        LspActor,
                        super::lsp::LspActorArgs {
                            documents: Arc::clone(&new_docs),
                        },
                        supervisor_cell,
                    )
                    .await
                    {
                        Ok((new_ref, _)) => {
                            state.child_ids.lsp_id = new_ref.get_id();
                            state.lsp_ref = new_ref;
                            state.lsp_documents = new_docs;
                            eprintln!("spectral supervisor: LspActor restarted");
                        }
                        Err(e) => {
                            eprintln!("spectral supervisor: failed to restart LspActor: {}", e);
                        }
                    }
                } else if dead_id == state.child_ids.compiler_id {
                    eprintln!(
                        "spectral supervisor: CompilerActor crashed ({}), restarting",
                        panic_msg
                    );
                    match Actor::spawn_linked(
                        child_name(&state.name_prefix, "compiler"),
                        CompilerActor,
                        (),
                        supervisor_cell,
                    )
                    .await
                    {
                        Ok((new_ref, _)) => {
                            state.child_ids.compiler_id = new_ref.get_id();
                            state.compiler_ref = new_ref;
                            eprintln!("spectral supervisor: CompilerActor restarted");
                        }
                        Err(e) => {
                            eprintln!("spectral supervisor: failed to restart CompilerActor: {}", e);
                        }
                    }
                } else if dead_id == state.child_ids.cascade_id {
                    eprintln!(
                        "spectral supervisor: CascadeActor crashed ({}), restarting",
                        panic_msg
                    );
                    match reopen_db(&state.db_path, &state.db_schema, state.db_precision, state.db_max_bytes) {
                        Ok(db) => {
                            match Actor::spawn_linked(
                                child_name(&state.name_prefix, "cascade"),
                                CascadeActor,
                                super::cascade::CascadeActorArgs { db, interval: None },
                                supervisor_cell,
                            )
                            .await
                            {
                                Ok((new_ref, _)) => {
                                    state.child_ids.cascade_id = new_ref.get_id();
                                    state.cascade_ref = new_ref;
                                    eprintln!("spectral supervisor: CascadeActor restarted");
                                }
                                Err(e) => {
                                    eprintln!("spectral supervisor: failed to restart CascadeActor: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("spectral supervisor: failed to reopen db for cascade: {}", e);
                        }
                    }
                } else {
                    eprintln!(
                        "spectral supervisor: unknown actor {} crashed: {}",
                        dead_id, panic_msg
                    );
                }
            }
            SupervisionEvent::ActorTerminated(who, _, reason) => {
                let dead_id = who.get_id();
                if dead_id == state.child_ids.mcp_id {
                    eprintln!(
                        "spectral supervisor: McpActor terminated ({:?}), stopping supervisor",
                        reason
                    );
                    myself.stop(reason);
                } else {
                    eprintln!(
                        "spectral supervisor: child {} terminated ({:?})",
                        dead_id, reason
                    );
                }
            }
            _ => {}
        }
        Ok(())
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Reopen SpectralDb from the given path. Used for restart recovery.
fn reopen_db(
    db_path: &PathBuf,
    schema: &str,
    precision: f64,
    max_bytes: usize,
) -> Result<SpectralDb, String> {
    SpectralDb::open(db_path, schema, precision, max_bytes)
        .map_err(|e| format!("reopen SpectralDb: {}", e))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sel::mcp::compiler::{CompileResult, CompilerMsg};
    use crate::sel::mcp::server::ToolCall;

    const SCHEMA: &str = "grammar @memory {\n  type = node | edge | eigenboard\n}";

    fn open_test_db() -> (tempfile::TempDir, SpectralDb, PathBuf) {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let db_path = dir.path().to_path_buf();
        let db = SpectralDb::open(&db_path, SCHEMA, 1e-6, 5_000_000)
            .expect("failed to open SpectralDb");
        (dir, db, db_path)
    }

    /// Spawn a supervisor without a registered name (avoids ractor name collisions in tests).
    async fn spawn_test_supervisor(
        db: SpectralDb,
        db_path: PathBuf,
    ) -> ActorRef<SupervisorMsg> {
        let (actor_ref, _handle) = Actor::spawn(
            None,
            SpectralSupervisor,
            SupervisorArgs {
                db,
                db_path,
                db_schema: SCHEMA.to_string(),
                db_precision: 1e-6,
                db_max_bytes: 5_000_000,
                name_prefix: None,
            },
        )
        .await
        .expect("supervisor spawn failed");
        actor_ref
    }

    #[tokio::test]
    async fn supervisor_spawns_all_children() {
        let (_dir, db, db_path) = open_test_db();

        let supervisor_ref = spawn_test_supervisor(db, db_path).await;

        // Verify all children are alive by querying their refs
        let mcp_ref: ActorRef<McpMsg> =
            ractor::call!(supervisor_ref, SupervisorMsg::GetMcpRef)
                .expect("get mcp ref failed");

        let (lsp_ref, _docs): (ActorRef<LspMsg>, _) =
            ractor::call!(supervisor_ref, SupervisorMsg::GetLspRef)
                .expect("get lsp ref failed");

        let compiler_ref: ActorRef<CompilerMsg> =
            ractor::call!(supervisor_ref, SupervisorMsg::GetCompilerRef)
                .expect("get compiler ref failed");

        let cascade_ref: ActorRef<CascadeMsg> =
            ractor::call!(supervisor_ref, SupervisorMsg::GetCascadeRef)
                .expect("get cascade ref failed");

        // Verify MCP actor can route a tool call
        let result: serde_json::Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "memory_status".to_string(),
                    arguments: serde_json::json!({}),
                },
                reply,
            )
        )
        .expect("mcp call failed");
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("nodes: 0"), "memory should work via supervisor, got: {}", text);

        // Verify compiler is alive
        let compile_result: CompileResult = ractor::call!(
            compiler_ref,
            |reply| CompilerMsg::Compile {
                source: "type x = a | b".to_string(),
                reply,
            }
        )
        .expect("compiler call failed");
        assert!(compile_result.success);

        // Verify cascade is alive
        let changed: bool = ractor::call!(cascade_ref, CascadeMsg::RunCascade)
            .expect("cascade call failed");
        assert!(!changed);

        // Verify LSP is alive
        lsp_ref
            .cast(LspMsg::DidOpen {
                uri: "file:///test.mirror".to_string(),
                source: "type a = x".to_string(),
            })
            .expect("lsp cast failed");

        supervisor_ref.stop(None);
    }

    #[tokio::test]
    async fn supervisor_children_respond_to_messages() {
        let (_dir, db, db_path) = open_test_db();

        let supervisor_ref = spawn_test_supervisor(db, db_path).await;

        // Get compiler ref and compile something
        let compiler_ref: ActorRef<CompilerMsg> =
            ractor::call!(supervisor_ref, SupervisorMsg::GetCompilerRef)
                .expect("get compiler ref failed");

        let result: CompileResult = ractor::call!(
            compiler_ref,
            |reply| CompilerMsg::Compile {
                source: "grammar @test { type = a | b }".to_string(),
                reply,
            }
        )
        .expect("compile call failed");
        assert!(result.success, "grammar should compile");

        // Get cascade ref and run cascade
        let cascade_ref: ActorRef<CascadeMsg> =
            ractor::call!(supervisor_ref, SupervisorMsg::GetCascadeRef)
                .expect("get cascade ref failed");

        let changed: bool = ractor::call!(cascade_ref, CascadeMsg::RunCascade)
            .expect("cascade run failed");
        assert!(!changed, "empty graph should not cascade");

        supervisor_ref.stop(None);
    }
}
