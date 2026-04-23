//! MCP subsystem — actors for the Spectral MCP server.
//!
//! Each actor owns its resource. No shared mutexes. All access goes through messages.
//!
//! `start_mcp()` is the public entry point: spawns actors, runs the stdio loop.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Value};
use spectral_db::SpectralDb;

use self::memory::MemoryActor;
use self::server::{McpActor, McpMsg, ToolCall};
use self::tools::{resource_definitions, scan_grammars, tool_definitions};
use super::fate_actor::FateActor;

pub mod lsp;
pub mod memory;
pub mod server;
pub mod tools;

// ── Default schema for MCP memory ────────────────────────────────────

const MEMORY_SCHEMA: &str = "grammar @memory {\n  type = node | edge\n}";
const MEMORY_PRECISION: f64 = 1e-6;
const MEMORY_BYTES: usize = 5_000_000;

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

// ── MCP dispatch (actor-backed) ──────────────────────────────────────

fn dispatch_protocol(
    msg: &Value,
    actions: &[tools::GrammarAction],
    mcp_ref: &ractor::ActorRef<McpMsg>,
    runtime: &tokio::runtime::Runtime,
) -> Option<Value> {
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
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            let result = match tool_name {
                Some(name) => {
                    let call = ToolCall {
                        name: name.to_string(),
                        arguments,
                    };
                    // Block on actor dispatch
                    runtime.block_on(async {
                        match ractor::call!(mcp_ref, |reply| McpMsg::CallTool(call, reply)) {
                            Ok(result) => result,
                            Err(e) => json!({
                                "content": [{ "type": "text", "text": format!("actor error: {}", e) }],
                                "isError": true
                            }),
                        }
                    })
                }
                None => json!({
                    "content": [{ "type": "text", "text": "missing tool name" }],
                    "isError": true
                }),
            };
            Some(jsonrpc_result(id, result))
        }
        "resources/list" => {
            let id = id?;
            Some(jsonrpc_result(id, json!({ "resources": resource_definitions() })))
        }
        "resources/read" => {
            let id = id?;
            // Route memory://status through the actor
            let text = runtime.block_on(async {
                match ractor::call!(mcp_ref, |reply| McpMsg::CallTool(
                    ToolCall {
                        name: "memory_status".to_string(),
                        arguments: json!({}),
                    },
                    reply,
                )) {
                    Ok(result) => result["content"][0]["text"]
                        .as_str()
                        .unwrap_or("status unavailable")
                        .to_string(),
                    Err(_) => "status unavailable".to_string(),
                }
            });
            Some(jsonrpc_result(id, json!({
                "contents": [{
                    "uri": "memory://status",
                    "mimeType": "text/plain",
                    "text": text
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

// ── Public entry point ───────────────────────────────────────────────

/// Start the MCP server with full actor backing.
///
/// Opens SpectralDb at `project_path/.spectral/`, spawns MemoryActor,
/// FateActor, and McpActor, then runs the JSON-RPC stdio loop.
pub fn start_mcp(project_path: PathBuf) {
    eprintln!("spectral serve: starting MCP server (actor-backed)");
    eprintln!("  project: {}", project_path.display());

    let project_str = project_path.to_string_lossy().to_string();
    let actions = scan_grammars(&project_str);
    eprintln!("  grammars: {} actions", actions.len());

    // Ensure .spectral/ directory exists for the database
    let db_path = project_path.join(".spectral");
    if !db_path.exists() {
        if let Err(e) = std::fs::create_dir_all(&db_path) {
            eprintln!("spectral serve: failed to create .spectral/: {}", e);
            eprintln!("  falling back to stub mode");
            return;
        }
    }

    // Open SpectralDb
    let db = match SpectralDb::open(&db_path, MEMORY_SCHEMA, MEMORY_PRECISION, MEMORY_BYTES) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("spectral serve: failed to open SpectralDb: {}", e);
            eprintln!("  falling back to stub mode");
            return;
        }
    };

    // Build tokio runtime and spawn actors
    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    let mcp_ref = runtime.block_on(async {
        let memory_ref = MemoryActor::spawn_with_db(Some("memory".to_string()), db)
            .await
            .expect("failed to spawn MemoryActor");

        let fate_ref = FateActor::spawn_untrained(Some("fate".to_string()))
            .await
            .expect("failed to spawn FateActor");

        McpActor::spawn_with_refs(
            Some("mcp".to_string()),
            memory_ref,
            fate_ref,
        )
        .await
        .expect("failed to spawn McpActor")
    });

    eprintln!("  MCP server ready (stdio) — actor-backed");

    // JSON-RPC stdio loop
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

        if let Some(response) = dispatch_protocol(&msg, &actions, &mcp_ref, &runtime) {
            if let Ok(json_str) = serde_json::to_string(&response) {
                let _ = writeln!(stdout, "{}", json_str);
                let _ = stdout.flush();
            }
        }
    }

    // Cleanup
    runtime.block_on(async {
        mcp_ref.stop(None);
    });

    eprintln!("spectral serve: shutting down");
}
