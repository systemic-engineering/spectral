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

    const SCHEMA: &str = "grammar @memory {\n  type = node | edge | eigenboard\n}";

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
        let (_dir, db, db_path) = open_test_db();
        let supervisor = spawn_test_supervisor(db, db_path).await;

        // Get all actor refs from the supervisor — proves they are alive
        let mcp: ActorRef<McpMsg> =
            ractor::call!(supervisor, SupervisorMsg::GetMcpRef)
                .expect("failed to get McpActor ref");

        let (lsp, _docs): (ActorRef<LspMsg>, Arc<DashMap<String, String>>) =
            ractor::call!(supervisor, SupervisorMsg::GetLspRef)
                .expect("failed to get LspActor ref");

        let compiler: ActorRef<CompilerMsg> =
            ractor::call!(supervisor, SupervisorMsg::GetCompilerRef)
                .expect("failed to get CompilerActor ref");

        let cascade: ActorRef<CascadeMsg> =
            ractor::call!(supervisor, SupervisorMsg::GetCascadeRef)
                .expect("failed to get CascadeActor ref");

        // Verify MemoryActor is alive by routing through McpActor
        let text = call_tool(&mcp, "memory_status", json!({})).await;
        assert!(
            text.contains("nodes:"),
            "memory_status should return real stats, got: {}",
            text
        );

        // All six actor types are alive: Memory, Fate, LSP, Compiler, Cascade, MCP
        // (Fate + Memory are implicit — supervisor spawns them, verified through MCP routing)
        // The refs obtained above prove the actors are alive; if any had failed to
        // spawn, the supervisor's pre_start would have returned an error.
        //
        // Verify the remaining actors respond to messages:
        let _cascade_result: bool =
            ractor::call!(cascade, CascadeMsg::RunCascade).expect("cascade should be alive");
        let _compile_result = ractor::call!(
            compiler,
            |reply| CompilerMsg::Compile {
                source: "type x = a".to_string(),
                reply,
            }
        )
        .expect("compiler should be alive");
        let _lsp_diags: DocumentDiagnostics = ractor::call!(
            lsp,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///nonexistent".to_string(),
                reply,
            }
        )
        .expect("lsp should be alive");

        supervisor.stop(None);
    }

    // ── Test 2: memory_status returns real stats ─────────────────────

    #[tokio::test]
    async fn e2e_2_memory_status_returns_real_stats() {
        let (_dir, db, db_path) = open_test_db();
        let supervisor = spawn_test_supervisor(db, db_path).await;

        let mcp: ActorRef<McpMsg> =
            ractor::call!(supervisor, SupervisorMsg::GetMcpRef)
                .expect("failed to get McpActor ref");

        let text = call_tool(&mcp, "memory_status", json!({})).await;

        // Must contain real fields, not a stub
        assert!(text.contains("nodes: 0"), "expected nodes: 0, got: {}", text);
        assert!(text.contains("edges: 0"), "expected edges: 0, got: {}", text);
        assert!(text.contains("crystals: 0"), "expected crystals: 0, got: {}", text);
        assert!(
            !text.contains("not yet wired"),
            "should be real data, not stub, got: {}",
            text
        );

        supervisor.stop(None);
    }

    // ── Test 3: memory_store returns real OID ────────────────────────

    #[tokio::test]
    async fn e2e_3_memory_store_returns_real_oid() {
        let (_dir, db, db_path) = open_test_db();
        let supervisor = spawn_test_supervisor(db, db_path).await;

        let mcp: ActorRef<McpMsg> =
            ractor::call!(supervisor, SupervisorMsg::GetMcpRef)
                .expect("failed to get McpActor ref");

        let text = call_tool(
            &mcp,
            "memory_store",
            json!({ "node_type": "node", "content": "hello world" }),
        )
        .await;

        // Response should be "stored: <hex OID>"
        assert!(
            text.starts_with("stored: "),
            "expected 'stored: <oid>', got: {}",
            text
        );

        // Extract and validate the OID
        let oid = text.strip_prefix("stored: ").unwrap();
        assert!(
            !oid.is_empty(),
            "OID should not be empty"
        );
        // OIDs are hex strings from fragmentation's content addressing
        assert!(
            oid.chars().all(|c| c.is_ascii_hexdigit()),
            "OID should be hex, got: {}",
            oid
        );

        supervisor.stop(None);
    }

    // ── Test 4: memory_recall finds stored node ──────────────────────

    #[tokio::test]
    async fn e2e_4_memory_recall_finds_stored_node() {
        let (_dir, db, db_path) = open_test_db();
        let supervisor = spawn_test_supervisor(db, db_path).await;

        let mcp: ActorRef<McpMsg> =
            ractor::call!(supervisor, SupervisorMsg::GetMcpRef)
                .expect("failed to get McpActor ref");

        // Store two nodes so spectral proximity has something to find.
        // SpectralDb's `near` excludes the target itself, so we need
        // at least two nodes and query from one to find the other.
        let store_a = call_tool(
            &mcp,
            "memory_store",
            json!({ "node_type": "node", "content": "alpha data" }),
        )
        .await;
        let oid_a = store_a
            .strip_prefix("stored: ")
            .expect("store should return OID");

        let store_b = call_tool(
            &mcp,
            "memory_store",
            json!({ "node_type": "node", "content": "beta data" }),
        )
        .await;
        let oid_b = store_b
            .strip_prefix("stored: ")
            .expect("store should return OID");

        // Recall with a wide distance from node A — should find node B
        let recall_text = call_tool(
            &mcp,
            "memory_recall",
            json!({ "oid": oid_a, "distance": 10.0 }),
        )
        .await;

        // The recall should find node B (the other stored node)
        assert!(
            recall_text.contains(oid_b),
            "recall from A should find B (oid_a={}, oid_b={}), got: {}",
            oid_a,
            oid_b,
            recall_text
        );
        assert!(
            recall_text.contains("node"),
            "recall should show the node type, got: {}",
            recall_text
        );

        supervisor.stop(None);
    }

    // ── Test 5: memory_status reflects stored data ───────────────────

    #[tokio::test]
    async fn e2e_5_memory_status_reflects_stored_data() {
        let (_dir, db, db_path) = open_test_db();
        let supervisor = spawn_test_supervisor(db, db_path).await;

        let mcp: ActorRef<McpMsg> =
            ractor::call!(supervisor, SupervisorMsg::GetMcpRef)
                .expect("failed to get McpActor ref");

        // Store two nodes
        let _ = call_tool(
            &mcp,
            "memory_store",
            json!({ "node_type": "node", "content": "first" }),
        )
        .await;
        let _ = call_tool(
            &mcp,
            "memory_store",
            json!({ "node_type": "node", "content": "second" }),
        )
        .await;

        // Status should reflect the insertions
        let text = call_tool(&mcp, "memory_status", json!({})).await;
        assert!(
            text.contains("nodes: 2"),
            "expected nodes: 2 after storing two nodes, got: {}",
            text
        );

        supervisor.stop(None);
    }

    // ── Test 6: Diagnostics for .mirror source ───────────────────────

    #[tokio::test]
    async fn e2e_6_diagnostics_for_mirror_source() {
        let (_dir, db, db_path) = open_test_db();
        let supervisor = spawn_test_supervisor(db, db_path).await;

        let (lsp, _docs): (ActorRef<LspMsg>, _) =
            ractor::call!(supervisor, SupervisorMsg::GetLspRef)
                .expect("failed to get LspActor ref");

        // Open a .mirror file with an unknown token to generate diagnostics
        lsp.cast(LspMsg::DidOpen {
            uri: "file:///test.mirror".to_string(),
            source: "type color = red | blue\nwidget foo".to_string(),
        })
        .expect("cast failed");

        // Sync: get diagnostics back through the actor
        let diags: DocumentDiagnostics = ractor::call!(
            lsp,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///test.mirror".to_string(),
                reply,
            }
        )
        .expect("get diagnostics failed");

        assert_eq!(diags.uri, "file:///test.mirror");
        assert!(
            !diags.diagnostics.is_empty(),
            "should have diagnostics for source with unknown token"
        );

        // Verify diagnostics contain M-codes from mirror's loss_to_diagnostics
        let has_m_code = diags.diagnostics.iter().any(|d| {
            d.code
                .as_ref()
                .map(|c| c.starts_with('M'))
                .unwrap_or(false)
        });
        assert!(
            has_m_code,
            "diagnostics should contain M-codes, got: {:?}",
            diags.diagnostics.iter().map(|d| &d.code).collect::<Vec<_>>()
        );

        supervisor.stop(None);
    }

    // ── Test 7: CodeLens returns loss metrics ────────────────────────

    #[tokio::test]
    async fn e2e_7_code_lens_returns_loss_metrics() {
        let (_dir, db, db_path) = open_test_db();
        let supervisor = spawn_test_supervisor(db, db_path).await;

        let (lsp, _docs): (ActorRef<LspMsg>, _) =
            ractor::call!(supervisor, SupervisorMsg::GetLspRef)
                .expect("failed to get LspActor ref");

        // Open a .mirror file with type declarations
        lsp.cast(LspMsg::DidOpen {
            uri: "file:///lens.mirror".to_string(),
            source: "type color = red | blue\ntype shape = circle | square".to_string(),
        })
        .expect("cast failed");

        // Sync
        let _: DocumentDiagnostics = ractor::call!(
            lsp,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///lens.mirror".to_string(),
                reply,
            }
        )
        .expect("sync failed");

        // Request codeLenses
        let lenses: Vec<CodeLensEntry> = ractor::call!(
            lsp,
            |reply| LspMsg::GetCodeLenses {
                uri: "file:///lens.mirror".to_string(),
                reply,
            }
        )
        .expect("code lens failed");

        // Two type declarations = two code lenses
        assert_eq!(
            lenses.len(),
            2,
            "should have one lens per type declaration"
        );

        // Each lens has loss and coupling metrics
        for lens in &lenses {
            // loss_bits is a valid float (not NaN/Infinity for clean source)
            assert!(
                lens.loss_bits.is_finite(),
                "loss_bits should be finite, got: {}",
                lens.loss_bits
            );
            assert!(
                lens.coupling.is_finite(),
                "coupling should be finite, got: {}",
                lens.coupling
            );
        }

        supervisor.stop(None);
    }

    // ── Test 8: spectral_loss tool returns self-loss data ────────────

    #[tokio::test]
    async fn e2e_8_spectral_loss_returns_self_loss_data() {
        let (_dir, db, db_path) = open_test_db();
        let supervisor = spawn_test_supervisor(db, db_path).await;

        let mcp: ActorRef<McpMsg> =
            ractor::call!(supervisor, SupervisorMsg::GetMcpRef)
                .expect("failed to get McpActor ref");

        let (lsp, _docs): (ActorRef<LspMsg>, _) =
            ractor::call!(supervisor, SupervisorMsg::GetLspRef)
                .expect("failed to get LspActor ref");

        // Open a doc so there's file-level loss data
        lsp.cast(LspMsg::DidOpen {
            uri: "file:///loss.mirror".to_string(),
            source: "type x = a | b\ngarbage line".to_string(),
        })
        .expect("cast failed");

        // Sync
        let _: DocumentDiagnostics = ractor::call!(
            lsp,
            |reply| LspMsg::GetDiagnostics {
                uri: "file:///loss.mirror".to_string(),
                reply,
            }
        )
        .expect("sync failed");

        // Call spectral_loss through MCP
        let text = call_tool(&mcp, "spectral_loss", json!({})).await;

        // Response should contain self_loss and proposal_count
        assert!(
            text.contains("self_loss:"),
            "should contain self_loss field, got: {}",
            text
        );
        assert!(
            text.contains("proposals:"),
            "should contain proposals field, got: {}",
            text
        );

        // Parse the self_loss value — should be a valid float
        let self_loss_str = text
            .split("self_loss: ")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .expect("failed to extract self_loss value");
        let self_loss: f64 = self_loss_str
            .parse()
            .expect("self_loss should be a valid float");
        assert!(
            self_loss.is_finite(),
            "self_loss should be finite, got: {}",
            self_loss
        );

        supervisor.stop(None);
    }

    // ── Test 10: graph_query end-to-end ──────────────────────────────

    #[tokio::test]
    async fn graph_query_end_to_end() {
        let (_dir, db, db_path) = open_test_db();
        let supervisor = spawn_test_supervisor(db, db_path).await;

        let mcp: ActorRef<McpMsg> =
            ractor::call!(supervisor, SupervisorMsg::GetMcpRef)
                .expect("failed to get McpActor ref");

        // Store eigenboard nodes
        for (repo, fiedler, n, e) in [
            ("/identity", 0.0432, 47, 122),
            ("/spectral", 0.0615, 83, 201),
            ("/small", 0.0100, 10, 15),
        ] {
            let text = call_tool(
                &mcp,
                "memory_store",
                json!({
                    "node_type": "eigenboard",
                    "content": format!("repo:{} fiedler={:.4} nodes={} edges={}", repo, fiedler, n, e)
                }),
            )
            .await;
            assert!(
                text.starts_with("stored:"),
                "expected stored:, got: {}",
                text
            );
        }

        // Query: find eigenboards with fiedler > 0.04, sort desc, limit 1
        let text = call_tool(
            &mcp,
            "graph_query",
            json!({
                "query": "find eigenboard |> where fiedler > 0.04 |> sort by fiedler desc |> limit 1"
            }),
        )
        .await;

        assert!(
            text.contains("count: 1"),
            "expected count: 1, got: {}",
            text
        );
        assert!(
            text.contains("spectral"),
            "highest fiedler should be spectral, got: {}",
            text
        );
        assert!(
            text.contains("loss:"),
            "should report loss, got: {}",
            text
        );

        supervisor.stop(None);
    }

    // ── Test 9: Content addressing roundtrip ─────────────────────────

    #[tokio::test]
    async fn e2e_9_content_addressing_roundtrip() {
        let (_dir, db, db_path) = open_test_db();
        let supervisor = spawn_test_supervisor(db, db_path).await;

        let mcp: ActorRef<McpMsg> =
            ractor::call!(supervisor, SupervisorMsg::GetMcpRef)
                .expect("failed to get McpActor ref");

        let content = "the quick brown fox jumps over the lazy dog";

        // Store data → get OID
        let store_text = call_tool(
            &mcp,
            "memory_store",
            json!({ "node_type": "node", "content": content }),
        )
        .await;
        let oid = store_text
            .strip_prefix("stored: ")
            .expect("store should return OID");
        assert!(!oid.is_empty(), "OID should not be empty");

        // Store the same content again → should get the SAME OID
        // (content addressing is deterministic)
        let store_again_text = call_tool(
            &mcp,
            "memory_store",
            json!({ "node_type": "node", "content": content }),
        )
        .await;
        let oid2 = store_again_text
            .strip_prefix("stored: ")
            .expect("second store should return OID");
        assert_eq!(
            oid, oid2,
            "same content should produce same OID (content addressing)"
        );

        // Store a second, different node so we can recall via spectral proximity
        let store_other = call_tool(
            &mcp,
            "memory_store",
            json!({ "node_type": "node", "content": "different content entirely" }),
        )
        .await;
        let oid_other = store_other
            .strip_prefix("stored: ")
            .expect("other store should return OID");

        // Different content → different OID
        assert_ne!(
            oid, oid_other,
            "different content should produce different OID"
        );

        // Recall from oid → should find oid_other via spectral proximity
        let recall_text = call_tool(
            &mcp,
            "memory_recall",
            json!({ "oid": oid, "distance": 10.0 }),
        )
        .await;
        assert!(
            recall_text.contains(oid_other),
            "recall should find the other node, got: {}",
            recall_text
        );

        // Verify the full roundtrip: store→status shows the right count
        let status_text = call_tool(&mcp, "memory_status", json!({})).await;
        assert!(
            status_text.contains("nodes:"),
            "status should contain node count, got: {}",
            status_text
        );

        supervisor.stop(None);
    }

    // ── Test 10: spectral_index — full pipeline — — — — — — — — — — —

    #[tokio::test]
    async fn e2e_10_spectral_index_runs_full_pipeline() {
        let (_dir, db, db_path) = open_test_db();
        let supervisor = spawn_test_supervisor(db, db_path).await;

        let mcp: ActorRef<McpMsg> =
            ractor::call!(supervisor, SupervisorMsg::GetMcpRef)
                .expect("failed to get McpActor ref");

        // Use a real directory (spectral src) as the index target
        let path = env!("CARGO_MANIFEST_DIR");
        let text = call_tool(&mcp, "spectral_index", serde_json::json!({ "path": path })).await;

        // Output must contain all four pipeline stages
        assert!(text.contains("files:"), "expected file count, got: {}", text);
        assert!(text.contains("fiedler:"), "expected fiedler value, got: {}", text);
        assert!(text.contains("cascade:"), "expected cascade result, got: {}", text);
        assert!(text.contains("crystals:"), "expected crystal count, got: {}", text);

        // At least one eigenboard node was stored
        let status = call_tool(&mcp, "memory_status", serde_json::json!({})).await;
        assert!(
            !status.contains("nodes: 0"),
            "index should have stored at least one node, got: {}",
            status
        );

        supervisor.stop(None);
    }
}
