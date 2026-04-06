//! spectral serve — MCP JSON-RPC server over stdio.
//!
//! Holds a composed Lens over user + project spectral-db instances.
//! Exposes memory operations as MCP tools via the Model Context Protocol.
//! Grammar-driven: scans project directory for .conv files and generates
//! MCP tools from grammar actions.
//!
//! Protocol: JSON-RPC 2.0, newline-delimited, stdin/stdout.
//! Logs go to stderr.

use std::io::{self, BufRead, Write};
use std::path::Path;

use lens::types::{Distance, NodeData, NodeType};
use lens::Lens;
use mirror::parse::Parse;
use mirror::Vector;
use prism::Oid;
use serde_json::{json, Value};

use crate::memory;

// ── Grammar scanning ──────────────────────────────────────────────

/// An action extracted from a .conv grammar file.
struct GrammarAction {
    grammar_name: String, // "reed" (no @)
    action_name: String,  // "observe"
}

/// Scan a project directory for .conv files and extract grammar actions.
fn scan_grammars(project_path: &str) -> Vec<GrammarAction> {
    let mut actions = Vec::new();
    let mut conv_files = Vec::new();

    let project = Path::new(project_path);

    // Check project root for .conv files
    if let Ok(entries) = std::fs::read_dir(project) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("conv") {
                conv_files.push(path);
            }
        }
    }

    // Check conv/ subdirectory
    let conv_dir = project.join("conv");
    if conv_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&conv_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("conv") {
                    conv_files.push(path);
                }
            }
        }
    }

    conv_files.sort();

    for path in &conv_files {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "spectral serve: failed to read {}: {}",
                    path.display(),
                    e
                );
                continue;
            }
        };

        let ast = match Parse.trace(source).into_result() {
            Ok(tree) => tree,
            Err(e) => {
                eprintln!(
                    "spectral serve: parse error in {}: {}",
                    path.display(),
                    e
                );
                continue;
            }
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

        eprintln!(
            "spectral serve: loaded {}",
            path.display()
        );
    }

    actions
}

// ── Tool definitions ────────────────────────────────────────────────

/// Built-in tools that are always available regardless of grammars.
fn builtin_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "memory_recall",
            "description": "Find memories near a given node by spectral proximity",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "oid": {
                        "type": "string",
                        "description": "OID of the node to search near"
                    },
                    "distance": {
                        "type": "number",
                        "description": "Maximum spectral distance (0.0-1.0)",
                        "default": 0.5
                    }
                },
                "required": ["oid"]
            }
        }),
        json!({
            "name": "memory_crystallize",
            "description": "Promote a memory node to crystallized procedural memory (survives pressure shedding)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "oid": {
                        "type": "string",
                        "description": "OID of the node to crystallize"
                    }
                },
                "required": ["oid"]
            }
        }),
        json!({
            "name": "memory_status",
            "description": "Get spectral memory status: node count, edge count, pressure",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
    ]
}

/// Generate tool definitions from parsed grammar actions.
fn grammar_tool_definitions(actions: &[GrammarAction]) -> Vec<Value> {
    actions
        .iter()
        .map(|action| {
            let tool_name = format!("{}__{}", action.grammar_name, action.action_name);
            let description = format!("{} in @{}", action.action_name, action.grammar_name);
            json!({
                "name": tool_name,
                "description": description,
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Content for this action"
                        }
                    },
                    "required": ["content"]
                }
            })
        })
        .collect()
}

/// All tool definitions: grammar-derived + built-in.
fn tool_definitions(actions: &[GrammarAction]) -> Value {
    let mut tools = grammar_tool_definitions(actions);
    tools.extend(builtin_tool_definitions());
    Value::Array(tools)
}

// ── Resource definitions ────────────────────────────────────────────

fn resource_definitions() -> Value {
    json!([
        {
            "uri": "memory://context",
            "name": "Memory Context",
            "description": "Current spectral memory context for this project",
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

// ── Tool dispatch ───────────────────────────────────────────────────

/// Handle a grammar-derived action: store content with the action name as node type.
fn handle_grammar_action(lens: &Lens, action_name: &str, arguments: &Value) -> Value {
    let content = match arguments.get("content").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_result_error("missing required argument: content"),
    };

    let node_type = NodeType::new_unchecked(action_name);
    let data = NodeData::from_str(content);
    let beam = lens.store(node_type, data);
    lens.flush();

    if beam.is_lossless() {
        tool_result_text(&format!("stored as {}: {}", action_name, beam.result))
    } else {
        let reason = match &beam.recovered {
            Some(prism::Recovery::Failed { reason }) => reason.as_str(),
            _ => "unknown error",
        };
        tool_result_error(&format!("store failed: {}", reason))
    }
}

fn handle_memory_recall(lens: &Lens, arguments: &Value) -> Value {
    let oid_str = match arguments.get("oid").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_result_error("missing required argument: oid"),
    };
    let distance = arguments
        .get("distance")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    let beam = lens.recall(Oid::new(oid_str), Distance::new(distance));
    let oids = &beam.result;

    if oids.is_empty() {
        tool_result_text(&format!("no results within distance {}", distance))
    } else {
        let mut lines = Vec::new();
        for oid in oids {
            let read_beam = lens.read(oid.clone());
            match read_beam.result {
                Some(data) => {
                    let text = String::from_utf8_lossy(data.as_bytes());
                    lines.push(format!("{} {}", oid, text));
                }
                None => {
                    lines.push(format!("{}", oid));
                }
            }
        }
        tool_result_text(&lines.join("\n"))
    }
}

fn handle_memory_crystallize(lens: &Lens, arguments: &Value) -> Value {
    let oid_str = match arguments.get("oid").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return tool_result_error("missing required argument: oid"),
    };

    let beam = lens.crystallize(Oid::new(oid_str));
    lens.flush();

    if beam.is_lossless() {
        tool_result_text(&format!("crystallized: {}", oid_str))
    } else {
        let reason = match &beam.recovered {
            Some(prism::Recovery::Failed { reason }) => reason.as_str(),
            _ => "unknown error",
        };
        tool_result_error(&format!("crystallize failed: {}", reason))
    }
}

fn handle_memory_status(lens: &Lens) -> Value {
    let (nodes, edges) = lens.graph_stats();
    tool_result_text(&format!(
        "nodes: {}\nedges: {}\npressure: 0.0",
        nodes, edges
    ))
}

// ── Resource dispatch ───────────────────────────────────────────────

fn handle_resource_read(lens: &Lens, uri: &str) -> Value {
    match uri {
        "memory://context" => {
            let (nodes, edges) = lens.graph_stats();
            json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "text/plain",
                    "text": format!("spectral memory context\nnodes: {}\nedges: {}", nodes, edges)
                }]
            })
        }
        "memory://status" => {
            let (nodes, edges) = lens.graph_stats();
            json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "text/plain",
                    "text": format!("nodes: {}\nedges: {}\npressure: 0.0", nodes, edges)
                }]
            })
        }
        _ => json!({
            "contents": [{
                "uri": uri,
                "mimeType": "text/plain",
                "text": format!("unknown resource: {}", uri)
            }]
        }),
    }
}

// ── Main dispatch loop ──────────────────────────────────────────────

fn dispatch(msg: &Value, lens: &Lens, actions: &[GrammarAction]) -> Option<Value> {
    let method = msg.get("method")?.as_str()?;
    let id = msg.get("id");

    // Notifications have no id — don't respond
    let is_notification = id.is_none();

    match method {
        "initialize" => {
            let id = id?;
            Some(jsonrpc_result(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {},
                        "resources": {}
                    },
                    "serverInfo": {
                        "name": "spectral",
                        "version": "0.1.0"
                    }
                }),
            ))
        }

        "notifications/initialized" => {
            eprintln!("spectral serve: client initialized");
            None // notification — no response
        }

        "tools/list" => {
            let id = id?;
            Some(jsonrpc_result(
                id,
                json!({ "tools": tool_definitions(actions) }),
            ))
        }

        "tools/call" => {
            let id = id?;
            let empty = json!({});
            let params = msg.get("params").unwrap_or(&empty);
            let tool_name = params.get("name").and_then(|v| v.as_str());
            let empty_args = json!({});
            let arguments = params.get("arguments").unwrap_or(&empty_args);

            let result = match tool_name {
                Some("memory_recall") => handle_memory_recall(lens, arguments),
                Some("memory_crystallize") => handle_memory_crystallize(lens, arguments),
                Some("memory_status") => handle_memory_status(lens),
                Some(name) if name.contains("__") => {
                    let parts: Vec<&str> = name.splitn(2, "__").collect();
                    let action = parts[1];
                    handle_grammar_action(lens, action, arguments)
                }
                Some(name) => tool_result_error(&format!("unknown tool: {}", name)),
                None => tool_result_error("missing tool name"),
            };

            Some(jsonrpc_result(id, result))
        }

        "resources/list" => {
            let id = id?;
            Some(jsonrpc_result(
                id,
                json!({ "resources": resource_definitions() }),
            ))
        }

        "resources/read" => {
            let id = id?;
            let empty_params = json!({});
            let params = msg.get("params").unwrap_or(&empty_params);
            let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
            Some(jsonrpc_result(id, handle_resource_read(lens, uri)))
        }

        _ => {
            if is_notification {
                eprintln!("spectral serve: ignoring notification '{}'", method);
                None
            } else {
                let id = id?;
                Some(jsonrpc_error(id, -32601, &format!("method not found: {}", method)))
            }
        }
    }
}

/// Start the MCP server — JSON-RPC over stdio.
pub fn serve(project_path: &str) {
    eprintln!("spectral serve: starting MCP server");
    eprintln!("  project: {}", project_path);

    // Scan for .conv grammars and extract actions
    let actions = scan_grammars(project_path);
    eprintln!(
        "  grammars: {} actions from .conv files",
        actions.len()
    );
    for action in &actions {
        eprintln!(
            "    {}__{}: {} in @{}",
            action.grammar_name, action.action_name, action.action_name, action.grammar_name
        );
    }

    // Build grammar filter from scanned actions — the .conv file defines the allowed types
    let mut filter = lens::filter::GrammarFilter::new("spectral");
    // Add action names as allowed types
    for action in &actions {
        filter = filter.allow_type(&action.action_name);
    }
    // Also allow the base types from the hardcoded filters
    for base_type in &["file", "function", "decision", "observation", "test", "pattern",
                        "preference", "feedback", "reference", "fact"] {
        filter = filter.allow_type(base_type);
    }
    eprintln!("  filter: {} allowed types", filter.allowed_types().len());

    // Open lens with the grammar-derived filter
    let db_path = std::path::Path::new(project_path).join(".spectral");
    if !db_path.exists() {
        std::fs::create_dir_all(&db_path).ok();
    }
    let lens = match Lens::open(&db_path, filter, "spectral-serve", 1e-6, 50_000_000) { // TODO: make configurable via .spec
        Ok(l) => l,
        Err(e) => {
            eprintln!("spectral serve: failed to open lens: {}", e);
            // Fall back to user lens
            match memory::open_user_lens() {
                Some(l) => l,
                None => {
                    eprintln!("spectral serve: no graphs available");
                    std::process::exit(1);
                }
            }
        }
    };

    let (nodes, edges) = lens.graph_stats();
    eprintln!(
        "  graph: {} nodes, {} edges",
        nodes, edges
    );
    eprintln!("  MCP server ready (stdio)");

    // JSON-RPC loop: read one JSON object per line from stdin
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("spectral serve: stdin read error: {}", e);
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse JSON-RPC message
        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("spectral serve: malformed JSON: {} — line: {}", e, trimmed);
                continue;
            }
        };

        // Dispatch and respond
        if let Some(response) = dispatch(&msg, &lens, &actions) {
            match serde_json::to_string(&response) {
                Ok(json_str) => {
                    if let Err(e) = writeln!(stdout, "{}", json_str) {
                        eprintln!("spectral serve: stdout write error: {}", e);
                        break;
                    }
                    if let Err(e) = stdout.flush() {
                        eprintln!("spectral serve: stdout flush error: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("spectral serve: JSON serialize error: {}", e);
                }
            }
        }
    }

    eprintln!("spectral serve: shutting down");
}
