//! McpActor — routes JSON-RPC tool calls to the appropriate child actor.
//!
//! The McpActor is a router. It receives a tool name and arguments,
//! figures out which actor handles it, dispatches, and returns the result.
//! No business logic lives here — just dispatch.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use serde_json::{json, Value};

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
    // TODO: LspActor ref — tower-lsp has its own transport, needs careful wiring
}

// ── Actor ────────────────────────────────────────────────────────────

/// The McpActor: routes tool calls to child actors.
pub struct McpActor;

/// Arguments to spawn a McpActor.
pub struct McpActorArgs {
    pub memory: ActorRef<MemoryMsg>,
    pub fate: ActorRef<FateMsg>,
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
            McpActorArgs { memory, fate },
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
///
/// TODO: dispatch to MemoryActor, FateActor based on tool name prefix.
async fn dispatch_tool(name: &str, _arguments: &Value, _state: &McpState) -> Value {
    // Stub — all tools return "not yet wired" until dispatch is implemented
    tool_result_error(&format!("{}: not yet wired", name))
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
