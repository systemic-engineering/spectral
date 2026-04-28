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
    pub project_path: Option<std::path::PathBuf>,
}

// ── Actor ────────────────────────────────────────────────────────────

/// The McpActor: routes tool calls to child actors.
pub struct McpActor;

/// Arguments to spawn a McpActor.
pub struct McpActorArgs {
    pub memory: ActorRef<MemoryMsg>,
    pub fate: ActorRef<FateMsg>,
    pub lsp: Option<ActorRef<LspMsg>>,
    pub project_path: Option<std::path::PathBuf>,
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
            McpActorArgs { memory, fate, lsp: None, project_path: None },
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
            McpActorArgs { memory, fate, lsp: Some(lsp), project_path: None },
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
            project_path: args.project_path,
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
        "spectral_index" => dispatch_spectral_index(arguments, state).await,
        "spectral_loss" => dispatch_spectral_loss(state).await,
        "gestalt_detect" => dispatch_gestalt_detect(arguments, state).await,
        "graph_query" => dispatch_graph_query(arguments, state).await,
        _ => tool_result_error(&format!("{}: unknown tool", name)),
    }
}

/// memory_status → MemoryActor::Status
async fn dispatch_memory_status(state: &McpState) -> Value {
    match ractor::call!(state.memory, MemoryMsg::Status) {
        Ok(status) => {
            // Write status.json so CLI subcommands can read live state
            if let Some(ref project_path) = state.project_path {
                let status_json = serde_json::json!({
                    "nodes": status.node_count,
                    "edges": status.edge_count,
                    "crystals": status.crystals,
                    "cached": status.cached,
                    "queries": status.query_count,
                    "hot_paths": status.hot_paths,
                    "loss_bits": 0.0,
                    "growth_pct": 0.0
                });
                let status_path = project_path.join(".git/spectral/status.json");
                let _ = std::fs::write(&status_path, status_json.to_string());
            }
            tool_result_text(&format!(
                "nodes: {}, edges: {}, crystals: {}, cached: {}, queries: {}, hot_paths: {}",
                status.node_count,
                status.edge_count,
                status.crystals,
                status.cached,
                status.query_count,
                status.hot_paths,
            ))
        }
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
        Ok(Ok(oid)) => {
            // Flush to git immediately — every store reaches disk.
            // Graphs and git. Always.
            let _ = state.memory.cast(MemoryMsg::Flush);
            tool_result_text(&format!("stored: {}", oid))
        }
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
                // Flush after crystallization — crystals must reach git.
                let _ = state.memory.cast(MemoryMsg::Flush);
                tool_result_text(&format!("crystallized {} subgraphs", crystals.len()))
            }
        }
        Err(e) => tool_result_error(&format!("memory_crystallize failed: {}", e)),
    }
}

/// spectral_index — Traversal<File, Crystal>
///
/// Full pipeline: gestalt import (wide) → edge detection via cascade →
/// Fate tournament (narrow) → crystallization. The diamond shape of
/// meaning emerging from a repo.
async fn dispatch_spectral_index(arguments: &Value, state: &McpState) -> Value {
    // Resolve path: argument > project_path > cwd
    let path_str = arguments
        .get("path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| state.project_path.as_ref().map(|p| p.to_string_lossy().into_owned()))
        .unwrap_or_else(|| ".".to_string());

    let path = std::path::Path::new(&path_str);
    if !path.is_dir() {
        return tool_result_error(&format!("spectral_index: '{}' is not a directory", path_str));
    }

    // ── Stage 1: Gestalt import (wide) ──────────────────────────────
    let (graph, _files, breakdown) = gestalt::graph::build_concept_graph(path);
    let profile = gestalt::eigenvalue::eigenvalue_profile(&graph);

    let mut out = Vec::new();
    out.push(format!("indexed: {}", path_str));
    out.push(format!(
        "  files:   {} (md:{} code:{} config:{} mirror:{})",
        breakdown.total(),
        breakdown.markdown,
        breakdown.code,
        breakdown.config,
        breakdown.mirror,
    ));
    out.push(format!(
        "  graph:   {} nodes, {} edges",
        graph.nodes.len(),
        graph.edges.len()
    ));

    let profile_oid = if !profile.is_dark() {
        out.push(format!("  fiedler: {:.4}", profile.fiedler_value()));
        let oid = profile.oid();
        // Persist eigenboard node (wide: covers full file set)
        let content = format!(
            "repo:{} fiedler={:.4} nodes={} edges={} oid={}",
            path_str,
            profile.fiedler_value(),
            graph.nodes.len(),
            graph.edges.len(),
            oid,
        );
        let _ = ractor::call!(
            state.memory,
            MemoryMsg::Store,
            "eigenboard".to_string(),
            content.into_bytes()
        );
        Some(oid)
    } else {
        out.push("  fiedler: dark (no connectivity)".to_string());
        None
    };

    // ── Stage 2: Cascade — recompute dirty spectral hashes ─────────
    let cascade_changed = match ractor::call!(state.memory, MemoryMsg::RunCascade) {
        Ok(changed) => changed,
        Err(e) => {
            out.push(format!("  cascade: failed ({})", e));
            false
        }
    };
    out.push(format!(
        "  cascade: {}",
        if cascade_changed { "settled (new edges)" } else { "stable" }
    ));

    // ── Stage 3: Content ingest — tokenize nodes, discover coincidence edges ──
    let mut total_coincidence = 0usize;
    for node_type in &["observation", "node", "eigenboard"] {
        if let Ok(Ok(count)) = ractor::call!(
            state.memory,
            MemoryMsg::IngestAll,
            node_type.to_string()
        ) {
            total_coincidence += count;
        }
    }
    if total_coincidence > 0 {
        out.push(format!("  ingest:  {} coincidence edges", total_coincidence));
    }

    // ── Stage 4: Crystallization (diamond tip) ───────────────────────
    let crystal_count = match ractor::call!(state.memory, MemoryMsg::Crystallize) {
        Ok(crystals) => crystals.len(),
        Err(_) => 0,
    };
    out.push(format!("  crystals: {}", crystal_count));

    if let Some(oid) = profile_oid {
        out.push(format!("  oid:     {}", oid));
    }

    // ── Persist graph summary + profile to .git/spectral/ ──────────────
    // The graph must survive process exit. Write via graph_cache for format
    // convergence: CLI and MCP write the same JSON shape with dir_hash.
    let resolved_path = std::path::Path::new(&path_str);
    match crate::apache2::graph_cache::write_graph_cache(resolved_path, &graph, &profile, &breakdown) {
        Ok(()) => out.push("  persisted: graph.json + profile.json".to_string()),
        Err(e) => out.push(format!("  persist failed: {}", e)),
    }

    // Flush — persist to git-backed store
    let _ = state.memory.cast(MemoryMsg::Flush);

    tool_result_text(&out.join("\n"))
}

/// spectral_loss → LspActor::GetLossReport
async fn dispatch_spectral_loss(state: &McpState) -> Value {
    match &state.lsp {
        Some(lsp_ref) => {
            match ractor::call!(lsp_ref, |reply| LspMsg::GetLossReport { reply }) {
                Ok(report) => format_loss_report(&report),
                Err(e) => tool_result_error(&format!("spectral_loss failed: {}", e)),
            }
        }
        None => tool_result_text("spectral_loss: LSP actor not connected (no loss data available)"),
    }
}

/// Format a LossReport into MCP tool result text.
fn format_loss_report(report: &LossReport) -> Value {
    let mut lines = Vec::new();

    lines.push(format!(
        "self_loss: {:.4} bits (proposals: {}, accepted: {}, rejected: {})",
        report.self_loss, report.proposal_count, report.accepted_count, report.rejected_count
    ));

    if report.files.is_empty() {
        lines.push("no files open".to_string());
    } else {
        lines.push(format!("files: {}", report.files.len()));
        for file in &report.files {
            lines.push(format!(
                "  {} — loss: {:.1} bits, diagnostics: {}, gutter: {}",
                file.uri,
                file.total_loss_bits,
                file.diagnostic_count,
                file.luminosity.as_str(),
            ));
        }
    }

    tool_result_text(&lines.join("\n"))
}

/// gestalt_detect — run gestalt auto-detection on a directory, persist results to spectral-db
async fn dispatch_gestalt_detect(arguments: &Value, state: &McpState) -> Value {
    let path_str = match arguments.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return tool_result_error("gestalt_detect: missing 'path' argument"),
    };
    let path = std::path::Path::new(path_str);
    if !path.is_dir() {
        return tool_result_error(&format!("gestalt_detect: '{}' is not a directory", path_str));
    }

    let cached = crate::apache2::graph_cache::load_or_build(path);
    let graph = &cached.graph;
    let profile = &cached.profile;
    let breakdown = &cached.breakdown;

    let mut lines = Vec::new();
    lines.push(format!(
        "total: {} files (md:{} code:{} config:{} asset:{} gestalt:{} mirror:{} other:{})",
        breakdown.total(),
        breakdown.markdown,
        breakdown.code,
        breakdown.config,
        breakdown.asset,
        breakdown.gestalt_native,
        breakdown.mirror,
        breakdown.other,
    ));
    lines.push(format!("graph: {} nodes, {} edges", graph.nodes.len(), graph.edges.len()));

    if !profile.is_dark() {
        lines.push(format!("fiedler: {:.4}", profile.fiedler_value()));
        let profile_str: String = profile.values
            .iter()
            .map(|v| format!("{:.4}", v))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("eigenvalues: [{}]", profile_str));
        lines.push(format!("profile_oid: {}", profile.oid()));

        // Persist eigenvalue profile to spectral-db
        let eigenboard_content = format!(
            "repo:{} fiedler={:.4} nodes={} edges={} oid={}",
            path_str,
            profile.fiedler_value(),
            graph.nodes.len(),
            graph.edges.len(),
            profile.oid(),
        );
        let _ = ractor::call!(
            state.memory,
            MemoryMsg::Store,
            "eigenboard".to_string(),
            eigenboard_content.into_bytes()
        );
    } else {
        lines.push("profile: dark (no connectivity)".to_string());
    }

    tool_result_text(&lines.join("\n"))
}


/// graph_query → MemoryActor::QueryFull
async fn dispatch_graph_query(arguments: &Value, state: &McpState) -> Value {
    let query = match arguments.get("query").and_then(|v| v.as_str()) {
        Some(q) => q.to_string(),
        None => return tool_result_error("graph_query: missing 'query' argument"),
    };

    match ractor::call!(state.memory, MemoryMsg::QueryFull, query) {
        Ok(Ok(response)) => {
            let nodes_json: Vec<Value> = response
                .nodes
                .iter()
                .map(|n| {
                    serde_json::json!({
                        "oid": n.oid,
                        "type": n.node_type,
                        "data": n.data,
                    })
                })
                .collect();
            tool_result_text(&format!(
                "count: {}, loss: {:.4} bits\nnodes: {}",
                response.count,
                response.loss,
                serde_json::to_string_pretty(&nodes_json).unwrap_or_default(),
            ))
        }
        Ok(Err(e)) => tool_result_error(&format!("graph_query failed: {}", e)),
        Err(e) => tool_result_error(&format!("graph_query actor error: {}", e)),
    }
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

    const SCHEMA: &str = "grammar @memory {\n  type = node | edge | eigenboard\n}";

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
    async fn mcp_memory_store_flushes_to_git() {
        let (dir, db) = open_test_db();
        let db_path = dir.path().to_path_buf();
        let memory_ref = MemoryActor::spawn_with_db(None, db)
            .await
            .expect("spawn memory failed");
        let fate_ref = FateActor::spawn_untrained(None)
            .await
            .expect("spawn fate failed");
        let mcp_ref = McpActor::spawn_with_refs(None, memory_ref.clone(), fate_ref.clone())
            .await
            .expect("spawn mcp failed");

        // Store a node via MCP
        let result: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "memory_store".to_string(),
                    arguments: json!({
                        "node_type": "node",
                        "content": "persistence test"
                    }),
                },
                reply,
            )
        )
        .expect("call failed");

        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("stored: "), "got: {}", text);

        // Sync: wait for flush to complete via a status call
        let _: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "memory_status".to_string(),
                    arguments: json!({}),
                },
                reply,
            )
        )
        .expect("status call failed");

        // The graph tree commit at refs/spectral/head must exist after store
        let head_ref_path = db_path.join(".git/refs/spectral/head");
        let packed_refs = db_path.join(".git/packed-refs");
        let has_head = head_ref_path.exists()
            || (packed_refs.exists()
                && std::fs::read_to_string(&packed_refs)
                    .unwrap_or_default()
                    .contains("refs/spectral/head"));
        assert!(
            has_head,
            "refs/spectral/head must exist after memory_store — store must flush to git"
        );

        // Verify: reopen SpectralDb at same path — node must survive
        mcp_ref.stop(None);
        memory_ref.stop(None);
        fate_ref.stop(None);

        // Give actors time to stop
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let db2 = SpectralDb::open(&db_path, SCHEMA, 1e-6, 5_000_000)
            .expect("reopen failed");
        let (node_count, _edge_count) = db2.graph_stats();
        assert!(
            node_count >= 1,
            "node must survive reopen after memory_store, got {} nodes",
            node_count
        );
    }

    #[tokio::test]
    async fn mcp_spectral_index_persists_graph_to_git() {
        // Create a directory with files to index
        let project = tempfile::tempdir().expect("failed to create project dir");
        let project_path = project.path().to_path_buf();
        std::fs::create_dir_all(project_path.join(".git/spectral")).unwrap();
        std::fs::write(project_path.join("readme.md"), "# Test\n\nContent.\n").unwrap();
        let sub = project_path.join("src");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("lib.rs"), "pub fn main() {}\n").unwrap();

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

        // Run spectral_index on the project directory
        let result: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "spectral_index".to_string(),
                    arguments: json!({ "path": project_path.to_str().unwrap() }),
                },
                reply,
            )
        )
        .expect("call failed");

        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("indexed:"), "should contain indexed, got: {}", text);

        // Graph must be persisted to .git/spectral/contexts/graph.json
        let graph_path = project_path.join(".git/spectral/contexts/graph.json");
        assert!(
            graph_path.exists(),
            "graph.json must be written after spectral_index"
        );

        // Profile must be persisted
        let profile_path = project_path.join(".git/spectral/contexts/profile.json");
        assert!(
            profile_path.exists(),
            "profile.json must be written after spectral_index"
        );

        // graph.json must be valid JSON and contain nodes + dir_hash
        let graph_content = std::fs::read_to_string(&graph_path).unwrap();
        let graph_val: serde_json::Value = serde_json::from_str(&graph_content)
            .expect("graph.json must be valid JSON");
        assert!(
            graph_val.get("nodes").is_some(),
            "graph.json must contain 'nodes' field"
        );
        assert!(
            graph_val.get("dir_hash").is_some(),
            "graph.json must contain 'dir_hash' for staleness check"
        );
        assert!(
            graph_val.get("breakdown").is_some(),
            "graph.json must contain 'breakdown' for CLI convergence"
        );

        mcp_ref.stop(None);
        memory_ref.stop(None);
        fate_ref.stop(None);
    }

    #[tokio::test]
    async fn mcp_index_cache_convergence_with_cli() {
        // After MCP writes graph.json, CLI's load_or_build should read it
        let project = tempfile::tempdir().expect("failed to create project dir");
        let project_path = project.path().to_path_buf();
        std::fs::create_dir_all(project_path.join(".git/spectral")).unwrap();
        std::fs::write(project_path.join("readme.md"), "# Hello\n\nWorld.\n").unwrap();

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

        // MCP index writes the cache
        let _: Value = ractor::call!(
            mcp_ref,
            |reply| McpMsg::CallTool(
                ToolCall {
                    name: "spectral_index".to_string(),
                    arguments: json!({ "path": project_path.to_str().unwrap() }),
                },
                reply,
            )
        )
        .expect("call failed");

        // CLI's load_or_build should use the cache (from_cache = true)
        let cached = crate::apache2::graph_cache::load_or_build(&project_path);
        assert!(
            cached.from_cache,
            "CLI must read MCP-written cache (from_cache should be true)"
        );
        assert!(
            cached.graph.nodes.len() > 0,
            "cached graph should have nodes"
        );

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
