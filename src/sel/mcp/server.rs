//! McpActor — routes JSON-RPC tool calls to the appropriate child actor.
//!
//! The McpActor is a router. It receives a tool name and arguments,
//! figures out which actor handles it, dispatches, and returns the result.
//! No business logic lives here — just dispatch.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use serde_json::{json, Value};

use super::lsp::{LspMsg, LossReport};
use super::memory::MemoryMsg;
use crate::sel::fate_actor::FateMsg;

// ── Messages ─────────────────────────────────────────────────────────

/// A parsed tool call request for the McpActor to route.
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

/// Messages the McpActor can receive.
pub enum McpMsg {
    /// Execute a tool call and return the JSON result.
    CallTool(ToolCall, ractor::RpcReplyPort<Value>),
}

// ── Actor state ──────────────────────────────────────────────────────

/// The McpActor's persistent state: refs to child actors.
pub struct McpState {
    pub memory: ActorRef<MemoryMsg>,
    pub fate: ActorRef<FateMsg>,
    pub lsp: Option<ActorRef<LspMsg>>,
}

// ── Actor ────────────────────────────────────────────────────────────

/// The McpActor: routes tool calls to child actors.
pub struct McpActor;

/// Arguments to spawn a McpActor.
pub struct McpActorArgs {
    pub memory: ActorRef<MemoryMsg>,
    pub fate: ActorRef<FateMsg>,
    pub lsp: Option<ActorRef<LspMsg>>,
}

impl McpActor {
    /// Spawn a McpActor with refs to child actors.
    pub async fn spawn_with_refs(
        name: Option<String>,
        memory: ActorRef<MemoryMsg>,
        fate: ActorRef<FateMsg>,
    ) -> Result<ActorRef<McpMsg>, ractor::SpawnErr> {
        let (actor_ref, _handle) = Actor::spawn(
            name,
            McpActor,
            McpActorArgs { memory, fate, lsp: None },
        )
        .await?;
        Ok(actor_ref)
    }

    /// Spawn a McpActor with refs to child actors, including LspActor.
    pub async fn spawn_with_lsp(
        name: Option<String>,
        memory: ActorRef<MemoryMsg>,
        fate: ActorRef<FateMsg>,
        lsp: ActorRef<LspMsg>,
    ) -> Result<ActorRef<McpMsg>, ractor::SpawnErr> {
        let (actor_ref, _handle) = Actor::spawn(
            name,
            McpActor,
            McpActorArgs { memory, fate, lsp: Some(lsp) },
        )
        .await?;
        Ok(actor_ref)
    }
}

#[ractor::async_trait]
impl Actor for McpActor {
    type Msg = McpMsg;
    type State = McpState;
    type Arguments = McpActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: McpActorArgs,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(McpState {
            memory: args.memory,
            fate: args.fate,
            lsp: args.lsp,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            McpMsg::CallTool(tool_call, reply) => {
                let result = dispatch_tool(&tool_call.name, &tool_call.arguments, state).await;
                let _ = reply.send(result);
            }
        }
        Ok(())
    }
}

// ── Dispatch ─────────────────────────────────────────────────────────

/// Route a tool call to the appropriate child actor.
async fn dispatch_tool(name: &str, arguments: &Value, state: &McpState) -> Value {
    match name {
        "memory_status" => dispatch_memory_status(state).await,
        "memory_store" => dispatch_memory_store(arguments, state).await,
        "memory_recall" => dispatch_memory_recall(arguments, state).await,
        "memory_crystallize" => dispatch_memory_crystallize(state).await,
        "spectral_loss" => dispatch_spectral_loss(state).await,
        _ => tool_result_error(&format!("{}: unknown tool", name)),
    }
}

/// memory_status → MemoryActor::Status
async fn dispatch_memory_status(state: &McpState) -> Value {
    match ractor::call!(state.memory, MemoryMsg::Status) {
        Ok(status) => tool_result_text(&format!(
            "nodes: {}, edges: {}, crystals: {}, cached: {}, queries: {}, hot_paths: {}",
            status.node_count,
            status.edge_count,
            status.crystals,
            status.cached,
            status.query_count,
            status.hot_paths,
        )),
        Err(e) => tool_result_error(&format!("memory_status failed: {}", e)),
    }
}

/// memory_store → MemoryActor::Store
async fn dispatch_memory_store(arguments: &Value, state: &McpState) -> Value {
    let node_type = match arguments.get("node_type").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => return tool_result_error("memory_store: missing 'node_type' argument"),
    };
    let content = match arguments.get("content").and_then(|v| v.as_str()) {
        Some(c) => c.as_bytes().to_vec(),
        None => return tool_result_error("memory_store: missing 'content' argument"),
    };

    match ractor::call!(state.memory, MemoryMsg::Store, node_type, content) {
        Ok(Ok(oid)) => tool_result_text(&format!("stored: {}", oid)),
        Ok(Err(e)) => tool_result_error(&format!("memory_store failed: {}", e)),
        Err(e) => tool_result_error(&format!("memory_store actor error: {}", e)),
    }
}

/// memory_recall → MemoryActor::Recall
async fn dispatch_memory_recall(arguments: &Value, state: &McpState) -> Value {
    let oid = match arguments.get("oid").and_then(|v| v.as_str()) {
        Some(o) => o.to_string(),
        None => return tool_result_error("memory_recall: missing 'oid' argument"),
    };
    let distance = arguments
        .get("distance")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    match ractor::call!(state.memory, MemoryMsg::Recall, oid, distance) {
        Ok(nodes) => {
            if nodes.is_empty() {
                tool_result_text("no nodes found within distance")
            } else {
                let lines: Vec<String> = nodes
                    .iter()
                    .map(|n| format!("{} ({}): d={:.4}", n.oid, n.node_type, n.distance))
                    .collect();
                tool_result_text(&lines.join("\n"))
            }
        }
        Err(e) => tool_result_error(&format!("memory_recall failed: {}", e)),
    }
}

/// memory_crystallize → MemoryActor::Crystallize
async fn dispatch_memory_crystallize(state: &McpState) -> Value {
    match ractor::call!(state.memory, MemoryMsg::Crystallize) {
        Ok(crystals) => {
            if crystals.is_empty() {
                tool_result_text("no subgraphs ready for crystallization")
            } else {
                tool_result_text(&format!("crystallized {} subgraphs", crystals.len()))
            }
        }
        Err(e) => tool_result_error(&format!("memory_crystallize failed: {}", e)),
    }
}

/// spectral_loss → LspActor::GetLossReport
async fn dispatch_spectral_loss(_state: &McpState) -> Value {
    todo!("tick-4: dispatch_spectral_loss")
}

// ── JSON helpers ─────────────────────────────────────────────────────

fn tool_result_text(text: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    })
}

fn tool_result_error(text: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": text
        }],
        "isError": true
    })
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::memory::MemoryActor;
    use crate::sel::fate_actor::FateActor;
    use crate::sel::mcp::lsp;
    use spectral_db::SpectralDb;

    const SCHEMA: &str = "grammar @memory {\n  type = node | edge\n}";

    fn open_test_db() -> (tempfile::TempDir, SpectralDb) {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let db = SpectralDb::open(dir.path(), SCHEMA, 1e-6, 5_000_000)
            .expect("failed to open SpectralDb");
        (dir, db)
    }

    async fn spawn_test_mcp() -> (
        tempfile::TempDir,
        ActorRef<McpMsg>,
        ActorRef<MemoryMsg>,
        ActorRef<FateMsg>,
    ) {
        let (_dir, db) = open_test_db();
        let memory_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn memory failed");
        let fate_ref = FateActor::spawn_untrained(None)
            .await
            .expect("spawn fate failed");
        let mcp_ref = McpActor::spawn_with_refs(None, memory_ref.clone(), fate_ref.clone())
            .await
            .expect("spawn mcp failed");
        (_dir, mcp_ref, memory_ref, fate_ref)
    }

    #[tokio::test]
    async fn mcp_routes_memory_status() {
        let (_dir, mcp_ref, memory_ref, fate_ref) = spawn_test_mcp().await;

        let result: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "memory_status".to_string(),
                    arguments: json!({}),
                },
                reply,
            )
        )
        .expect("call failed");

        // Should contain real stats, not "not yet wired"
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("nodes: 0"), "got: {}", text);
        assert!(text.contains("edges: 0"), "got: {}", text);
        assert!(text.contains("crystals: 0"), "got: {}", text);
        assert!(!text.contains("not yet wired"), "should be real data, got: {}", text);

        mcp_ref.stop(None);
        memory_ref.stop(None);
        fate_ref.stop(None);
    }

    #[tokio::test]
    async fn mcp_routes_memory_store_returns_oid() {
        let (_dir, mcp_ref, memory_ref, fate_ref) = spawn_test_mcp().await;

        let result: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "memory_store".to_string(),
                    arguments: json!({
                        "node_type": "node",
                        "content": "hello world"
                    }),
                },
                reply,
            )
        )
        .expect("call failed");

        // Should return a real OID
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("stored: "), "got: {}", text);
        assert!(!result.get("isError").is_some_and(|v| v.as_bool() == Some(true)), "should not be error");

        // Verify status now shows 1 node
        let status: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "memory_status".to_string(),
                    arguments: json!({}),
                },
                reply,
            )
        )
        .expect("call failed");
        let status_text = status["content"][0]["text"].as_str().unwrap();
        assert!(status_text.contains("nodes: 1"), "got: {}", status_text);

        mcp_ref.stop(None);
        memory_ref.stop(None);
        fate_ref.stop(None);
    }

    #[tokio::test]
    async fn mcp_routes_memory_store_invalid_type() {
        let (_dir, mcp_ref, memory_ref, fate_ref) = spawn_test_mcp().await;

        let result: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "memory_store".to_string(),
                    arguments: json!({
                        "node_type": "nonexistent",
                        "content": "data"
                    }),
                },
                reply,
            )
        )
        .expect("call failed");

        assert!(result["isError"].as_bool() == Some(true), "should be error for invalid type");

        mcp_ref.stop(None);
        memory_ref.stop(None);
        fate_ref.stop(None);
    }

    #[tokio::test]
    async fn mcp_routes_memory_store_missing_args() {
        let (_dir, mcp_ref, memory_ref, fate_ref) = spawn_test_mcp().await;

        let result: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "memory_store".to_string(),
                    arguments: json!({}),
                },
                reply,
            )
        )
        .expect("call failed");

        assert!(result["isError"].as_bool() == Some(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("missing"), "got: {}", text);

        mcp_ref.stop(None);
        memory_ref.stop(None);
        fate_ref.stop(None);
    }

    #[tokio::test]
    async fn mcp_routes_memory_recall_empty() {
        let (_dir, mcp_ref, memory_ref, fate_ref) = spawn_test_mcp().await;

        let result: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "memory_recall".to_string(),
                    arguments: json!({ "oid": "nonexistent" }),
                },
                reply,
            )
        )
        .expect("call failed");

        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("no nodes found"), "got: {}", text);

        mcp_ref.stop(None);
        memory_ref.stop(None);
        fate_ref.stop(None);
    }

    #[tokio::test]
    async fn mcp_routes_memory_crystallize_empty() {
        let (_dir, mcp_ref, memory_ref, fate_ref) = spawn_test_mcp().await;

        let result: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "memory_crystallize".to_string(),
                    arguments: json!({}),
                },
                reply,
            )
        )
        .expect("call failed");

        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("no subgraphs ready"), "got: {}", text);

        mcp_ref.stop(None);
        memory_ref.stop(None);
        fate_ref.stop(None);
    }

    #[tokio::test]
    async fn mcp_routes_spectral_loss_without_lsp() {
        let (_dir, mcp_ref, memory_ref, fate_ref) = spawn_test_mcp().await;

        let result: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "spectral_loss".to_string(),
                    arguments: json!({}),
                },
                reply,
            )
        )
        .expect("call failed");

        // Without LSP actor, should return a message saying not connected
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("not connected"), "got: {}", text);

        mcp_ref.stop(None);
        memory_ref.stop(None);
        fate_ref.stop(None);
    }

    #[tokio::test]
    async fn mcp_routes_spectral_loss_with_lsp() {
        let (_dir, db) = open_test_db();
        let memory_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn memory failed");
        let fate_ref = FateActor::spawn_untrained(None)
            .await
            .expect("spawn fate failed");
        let (lsp_ref, _docs): (ActorRef<LspMsg>, _) =
            lsp::LspActor::spawn_new(None)
                .await
                .expect("spawn lsp failed");

        // Open a doc so there's loss data
        lsp_ref
            .cast(LspMsg::DidOpen {
                uri: "file:///test.mirror".to_string(),
                source: "type color = red | blue\nwidget foo".to_string(),
            })
            .expect("cast failed");

        // Sync
        let _: lsp::DocumentDiagnostics = ractor::call!(
            lsp_ref,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///test.mirror".to_string(),
                reply,
            }
        )
        .expect("sync failed");

        let mcp_ref = McpActor::spawn_with_lsp(
            None,
            memory_ref.clone(),
            fate_ref.clone(),
            lsp_ref.clone(),
        )
        .await
        .expect("spawn mcp failed");

        let result: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "spectral_loss".to_string(),
                    arguments: json!({}),
                },
                reply,
            )
        )
        .expect("call failed");

        let text = result["content"][0]["text"].as_str().unwrap();
        // Should contain real loss data
        assert!(text.contains("self_loss:"), "got: {}", text);
        assert!(text.contains("file:///test.mirror"), "got: {}", text);
        assert!(text.contains("loss:"), "got: {}", text);

        mcp_ref.stop(None);
        lsp_ref.stop(None);
        memory_ref.stop(None);
        fate_ref.stop(None);
    }

    #[tokio::test]
    async fn mcp_unknown_tool_returns_error() {
        let (_dir, mcp_ref, memory_ref, fate_ref) = spawn_test_mcp().await;

        let result: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "bogus_tool".to_string(),
                    arguments: json!({}),
                },
                reply,
            )
        )
        .expect("call failed");

        assert!(result["isError"].as_bool() == Some(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("unknown tool"), "got: {}", text);

        mcp_ref.stop(None);
        memory_ref.stop(None);
        fate_ref.stop(None);
    }
}
