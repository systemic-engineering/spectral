//! Integration tests — Tick 5: end-to-end litmus test.
//!
//! Nine tests that prove the MCP is alive. Not individual actors in isolation,
//! but the full supervised system working together. Each test spawns the
//! SpectralSupervisor (which spawns all child actors), then exercises the
//! full data flow through the actor stack.
//!
//! If all nine pass, the thing breathes.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use dashmap::DashMap;
    use ractor::{Actor, ActorRef};
    use serde_json::json;
    use spectral_db::SpectralDb;

    use crate::sel::mcp::cascade::CascadeMsg;
    use crate::sel::mcp::compiler::CompilerMsg;
    use crate::sel::mcp::lsp::{CodeLensEntry, DocumentDiagnostics, LspMsg};
    use crate::sel::mcp::server::{McpMsg, ToolCall};
    use crate::sel::mcp::supervisor::{
        SpectralSupervisor, SupervisorArgs, SupervisorMsg,
    };

    const SCHEMA: &str = "grammar @memory {\n  type = node | edge\n}";

    /// Open a test SpectralDb in a temp directory.
    fn open_test_db() -> (tempfile::TempDir, SpectralDb, PathBuf) {
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let db_path = dir.path().to_path_buf();
        let db = SpectralDb::open(&db_path, SCHEMA, 1e-6, 5_000_000)
            .expect("failed to open SpectralDb");
        (dir, db, db_path)
    }

    /// Spawn a full supervisor tree without registered names (avoids ractor
    /// name collisions across parallel tests).
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

    /// Helper: call a tool through McpActor and return the text content.
    async fn call_tool(mcp: &ActorRef<McpMsg>, name: &str, args: serde_json::Value) -> String {
        let result: serde_json::Value = ractor::call!(
            mcp,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: name.to_string(),
                    arguments: args,
                },
                reply,
            )
        )
        .expect("tool call failed");
        result["content"][0]["text"]
            .as_str()
            .unwrap_or("<no text>")
            .to_string()
    }

    // ── Test 1: MCP server starts ────────────────────────────────────

    #[tokio::test]
    async fn e2e_1_mcp_server_starts() {
        todo!("wire: spawn supervisor, verify all actors alive")
    }

    // ── Test 2: memory_status returns real stats ─────────────────────

    #[tokio::test]
    async fn e2e_2_memory_status_returns_real_stats() {
        todo!("wire: call memory_status, verify real fields not stubs")
    }

    // ── Test 3: memory_store returns real OID ────────────────────────

    #[tokio::test]
    async fn e2e_3_memory_store_returns_real_oid() {
        todo!("wire: call memory_store, verify hex OID returned")
    }

    // ── Test 4: memory_recall finds stored node ──────────────────────

    #[tokio::test]
    async fn e2e_4_memory_recall_finds_stored_node() {
        todo!("wire: store two nodes, recall from one, find the other")
    }

    // ── Test 5: memory_status reflects stored data ───────────────────

    #[tokio::test]
    async fn e2e_5_memory_status_reflects_stored_data() {
        todo!("wire: store nodes, verify status count increases")
    }

    // ── Test 6: Diagnostics for .mirror source ───────────────────────

    #[tokio::test]
    async fn e2e_6_diagnostics_for_mirror_source() {
        todo!("wire: send DidOpen with bad source, verify M-coded diagnostics")
    }

    // ── Test 7: CodeLens returns loss metrics ────────────────────────

    #[tokio::test]
    async fn e2e_7_code_lens_returns_loss_metrics() {
        todo!("wire: send DidOpen with declarations, verify codeLens entries")
    }

    // ── Test 8: spectral_loss tool returns self-loss data ────────────

    #[tokio::test]
    async fn e2e_8_spectral_loss_returns_self_loss_data() {
        todo!("wire: call spectral_loss through MCP, verify self_loss float")
    }

    // ── Test 9: Content addressing roundtrip ─────────────────────────

    #[tokio::test]
    async fn e2e_9_content_addressing_roundtrip() {
        todo!("wire: store→OID→store same→same OID, recall via proximity")
    }
}
