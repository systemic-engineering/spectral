//! spectral serve — MCP server scaffold.
//!
//! Holds a composed Lens over user + project spectral-db instances.
//! Exposes memory operations as MCP tools.
//!
//! This is the scaffold — prints tool definitions and reads stdin
//! line-by-line. Full MCP JSON-RPC protocol in a follow-up task.

use std::io::BufRead;

use crate::memory;

/// Start the MCP server.
pub fn serve(project_path: &str) {
    eprintln!("spectral serve: starting MCP server");
    eprintln!("  project: {}", project_path);

    // Open lenses
    let user_lens = memory::open_user_lens();
    let project_lens = memory::open_project_lens(project_path);

    if user_lens.is_none() && project_lens.is_none() {
        eprintln!("spectral serve: no graphs available");
        std::process::exit(1);
    }

    eprintln!(
        "  user lens:    {}",
        if user_lens.is_some() { "active" } else { "none" }
    );
    eprintln!(
        "  project lens: {}",
        if project_lens.is_some() {
            "active"
        } else {
            "none"
        }
    );

    // MCP tool definitions
    let tools = serde_json::json!({
        "tools": [
            {
                "name": "memory_store",
                "description": "Store a fact, observation, or pattern in spectral memory",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "node_type": {
                            "type": "string",
                            "enum": ["fact", "observation", "decision", "pattern"]
                        },
                        "content": { "type": "string" }
                    },
                    "required": ["node_type", "content"]
                }
            },
            {
                "name": "memory_recall",
                "description": "Recall memories spectrally near a given node",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "oid": { "type": "string" },
                        "distance": { "type": "number", "default": 0.5 }
                    },
                    "required": ["oid"]
                }
            },
            {
                "name": "memory_crystallize",
                "description": "Promote a memory to crystallized procedural memory (survives pressure)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "oid": { "type": "string" }
                    },
                    "required": ["oid"]
                }
            },
            {
                "name": "memory_status",
                "description": "Get memory status: node count, pressure, crystals",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    });

    let tools_json = serde_json::to_string_pretty(&tools).unwrap();
    eprintln!("  tools: {}", tools_json);
    eprintln!("  MCP server ready (stdio)");
    eprintln!("  (full MCP JSON-RPC loop: next iteration)");

    // JSON-RPC stub — read stdin line-by-line
    let stdin = std::io::stdin();
    loop {
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let _trimmed = line.trim();
                // TODO: parse JSON-RPC and dispatch to lens operations
            }
            Err(_) => break,
        }
    }
}
