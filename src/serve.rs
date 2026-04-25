//! spectral serve — MCP JSON-RPC server over stdio.
//!
//! When the `sel` feature is enabled, delegates to `sel::mcp::start_mcp()`
//! which spawns real actors (MemoryActor, FateActor, McpActor) and routes
//! tool calls through the actor system.
//!
//! Without `sel`, runs the stub mode: protocol skeleton works, all tools
//! return "not yet wired."

/// Start the MCP server — JSON-RPC over stdio.
///
/// With `--features sel`: actor-backed, real dispatch.
/// Without: stub mode, protocol only.
pub fn serve(project_path: &str) {
    spawn_binary_watcher();

    #[cfg(feature = "sel")]
    {
        let path = std::path::PathBuf::from(project_path);
        spectral::sel::mcp::start_mcp(path);
    }

    #[cfg(not(feature = "sel"))]
    {
        serve_stub(project_path);
    }
}

#[cfg(test)]
mod tests {
    use super::binary_changed;

    #[test]
    fn binary_changed_false_for_current_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bin");
        std::fs::write(&path, b"v1").unwrap();
        let baseline = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert!(!binary_changed(&path, baseline));
    }

    #[test]
    fn binary_changed_true_after_rebuild() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bin");
        std::fs::write(&path, b"v1").unwrap();
        let baseline = std::fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, b"v2").unwrap();
        assert!(binary_changed(&path, baseline));
    }

    #[test]
    fn binary_changed_false_for_missing_file() {
        let baseline = std::time::SystemTime::UNIX_EPOCH;
        assert!(!binary_changed(std::path::Path::new("/nonexistent/bin"), baseline));
    }
}

// ── Stub mode (no sel feature) ───────────────────────────────────────

#[cfg(not(feature = "sel"))]
fn serve_stub(project_path: &str) {
    use std::io::{self, BufRead, Write};

    use mirror::parse::Parse;
    use mirror::Vector;
    use serde_json::{json, Value};

    // ── Grammar scanning ──────────────────────────────────────────────

    struct GrammarAction {
        grammar_name: String,
        action_name: String,
    }

    fn scan_grammars(project_path: &str) -> Vec<GrammarAction> {
        let mut actions = Vec::new();
        let mut files = Vec::new();

        let project = std::path::Path::new(project_path);

        if let Ok(entries) = std::fs::read_dir(project) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "conv" || ext == "mirror" {
                        files.push(path);
                    }
                }
            }
        }

        files.sort();

        for path in &files {
            let source = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let ast = match Parse.trace(source).into_result() {
                Ok(tree) => tree,
                Err(_) => continue,
            };

            for child in ast.children() {
                if child.data().is_decl("grammar") {
                    let raw_name = &child.data().value;
                    let grammar_name = raw_name.strip_prefix('@').unwrap_or(raw_name).to_string();

                    for grammar_child in child.children() {
                        if grammar_child.data().name == "action-def" {
                            let action_name = grammar_child.data().value.clone();
                            actions.push(GrammarAction {
                                grammar_name: grammar_name.clone(),
                                action_name,
                            });
                        }
                    }
                }
            }
        }

        actions
    }

    // ── JSON-RPC helpers ────────────────────────────────────────────────

    fn jsonrpc_result(id: &Value, result: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })
    }

    fn jsonrpc_error(id: &Value, code: i64, message: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
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

    // ── Tool definitions ────────────────────────────────────────────────

    fn builtin_tool_definitions() -> Vec<Value> {
        vec![
            json!({
                "name": "memory_recall",
                "description": "Find memories near a given node by spectral proximity (not yet wired)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "oid": { "type": "string", "description": "OID of the node to search near" },
                        "distance": { "type": "number", "description": "Maximum spectral distance", "default": 0.5 }
                    },
                    "required": ["oid"]
                }
            }),
            json!({
                "name": "memory_crystallize",
                "description": "Promote a memory node to crystallized procedural memory (not yet wired)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "oid": { "type": "string", "description": "OID of the node to crystallize" }
                    },
                    "required": ["oid"]
                }
            }),
            json!({
                "name": "memory_status",
                "description": "Get spectral memory status (not yet wired)",
                "inputSchema": { "type": "object", "properties": {} }
            }),
        ]
    }

    fn grammar_tool_definitions(actions: &[GrammarAction]) -> Vec<Value> {
        actions
            .iter()
            .map(|action| {
                let tool_name = format!("{}__{}", action.grammar_name, action.action_name);
                let description = format!("{} in @{} (not yet wired)", action.action_name, action.grammar_name);
                json!({
                    "name": tool_name,
                    "description": description,
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "content": { "type": "string", "description": "Content for this action" }
                        },
                        "required": ["content"]
                    }
                })
            })
            .collect()
    }

    fn tool_definitions(actions: &[GrammarAction]) -> Value {
        let mut tools = grammar_tool_definitions(actions);
        tools.extend(builtin_tool_definitions());
        Value::Array(tools)
    }

    fn resource_definitions() -> Value {
        json!([
            {
                "uri": "memory://context",
                "name": "Memory Context",
                "description": "Current spectral memory context",
                "mimeType": "text/plain"
            },
            {
                "uri": "memory://status",
                "name": "Memory Status",
                "description": "Spectral memory graph statistics",
                "mimeType": "text/plain"
            }
        ])
    }

    // ── Dispatch ────────────────────────────────────────────────────────

    fn dispatch(msg: &Value, actions: &[GrammarAction]) -> Option<Value> {
        let method = msg.get("method")?.as_str()?;
        let id = msg.get("id");
        let is_notification = id.is_none();

        match method {
            "initialize" => {
                let id = id?;
                Some(jsonrpc_result(
                    id,
                    json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {}, "resources": {} },
                        "serverInfo": { "name": "spectral", "version": "0.2.0" }
                    }),
                ))
            }
            "notifications/initialized" => {
                eprintln!("spectral serve: client initialized");
                None
            }
            "tools/list" => {
                let id = id?;
                Some(jsonrpc_result(id, json!({ "tools": tool_definitions(actions) })))
            }
            "tools/call" => {
                let id = id?;
                let empty = json!({});
                let params = msg.get("params").unwrap_or(&empty);
                let tool_name = params.get("name").and_then(|v| v.as_str());

                let result = match tool_name {
                    Some(name) => tool_result_error(&format!("{}: not yet wired (requires --features sel)", name)),
                    None => tool_result_error("missing tool name"),
                };
                Some(jsonrpc_result(id, result))
            }
            "resources/list" => {
                let id = id?;
                Some(jsonrpc_result(id, json!({ "resources": resource_definitions() })))
            }
            "resources/read" => {
                let id = id?;
                Some(jsonrpc_result(id, json!({
                    "contents": [{
                        "uri": "memory://status",
                        "mimeType": "text/plain",
                        "text": "spectral memory: not yet wired (requires --features sel)"
                    }]
                })))
            }
            _ => {
                if is_notification {
                    None
                } else {
                    let id = id?;
                    Some(jsonrpc_error(id, -32601, &format!("method not found: {}", method)))
                }
            }
        }
    }

    // ── Main loop ───────────────────────────────────────────────────────

    eprintln!("spectral serve: starting MCP server (stub mode — build with --features sel for actors)");
    eprintln!("  project: {}", project_path);

    let actions = scan_grammars(project_path);
    eprintln!("  grammars: {} actions", actions.len());
    eprintln!("  MCP server ready (stdio) — stub mode");

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("spectral serve: malformed JSON: {}", e);
                continue;
            }
        };

        if let Some(response) = dispatch(&msg, &actions) {
            if let Ok(json_str) = serde_json::to_string(&response) {
                let _ = writeln!(stdout, "{}", json_str);
                let _ = stdout.flush();
            }
        }
    }

    eprintln!("spectral serve: shutting down");
}
