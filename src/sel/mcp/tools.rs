//! MCP tool definitions — built-in tools + grammar-scanned tools.
//!
//! Harvested from serve.rs. Tool definitions are data, not behavior.
//! The McpActor uses tool names to route to the correct child actor.

use mirror::parse::Parse;
use mirror::Vector;
use serde_json::{json, Value};

// ── Grammar scanning ──────────────────────────────────────────────

/// An action extracted from a .mirror grammar file.
pub struct GrammarAction {
    pub grammar_name: String,
    pub action_name: String,
}

/// Scan a project directory for .mirror files and extract grammar actions.
pub fn scan_grammars(project_path: &str) -> Vec<GrammarAction> {
    let mut actions = Vec::new();
    let mut files = Vec::new();

    let project = std::path::Path::new(project_path);

    // Check project root
    if let Ok(entries) = std::fs::read_dir(project) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "mirror" {
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

// ── Tool definitions ────────────────────────────────────────────────

/// Built-in memory tools exposed via MCP.
pub fn builtin_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "memory_store",
            "description": "Store a node in spectral memory. Returns the content-addressed OID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "node_type": { "type": "string", "description": "Type of node (must match grammar schema)" },
                    "content": { "type": "string", "description": "Content to store" }
                },
                "required": ["node_type", "content"]
            }
        }),
        json!({
            "name": "memory_recall",
            "description": "Find memories near a given node by spectral proximity.",
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
            "description": "Promote settled subgraphs to crystallized procedural memory.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "memory_status",
            "description": "Get spectral memory status — node count, edge count, crystals.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "spectral_loss",
            "description": "Inspect the peer's self-knowledge: per-file loss breakdown, self-loss metric, and proposal acceptance stats. The honest gutter in data form.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "spectral_index",
            "description": "Traversal<File, Crystal> — full index pipeline: gestalt import (wide) -> edge detection -> Fate tournament (narrow) -> crystallization. The diamond shape of meaning emerging from a repo. Run on commit for continuous knowledge accumulation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path to index (defaults to current project)" }
                }
            }
        }),
        json!({
            "name": "gestalt_detect",
            "description": "Run gestalt auto-detection on a directory. Returns file counts by type, concept graph summary, and eigenvalue profile (spectral fingerprint). Works on any repo, no .mirror files required.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path to analyze" }
                },
                "required": ["path"]
            }
        }),
        // ── Git-native optics (memory_diff, memory_blame, memory_branch,
        //    memory_checkout, memory_thread, memory_cherrypick) ───────────
        json!({
            "name": "memory_diff",
            "description": "List nodes/edges/crystals/metadata added or removed between two refs on refs/spectral/HEAD. Defaults: from=HEAD~1, to=HEAD. Wraps `git diff-tree`. Returns structured JSON.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": { "type": "string", "description": "Source ref (default HEAD~1; e.g. 'HEAD~3', a branch, or an oid)" },
                    "to":   { "type": "string", "description": "Destination ref (default HEAD)" }
                }
            }
        }),
        json!({
            "name": "memory_blame",
            "description": "Return the chronological commit chain that touched nodes/{oid}/. Wraps `git log --follow refs/spectral/HEAD -- nodes/{oid}/`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "oid": { "type": "string", "description": "OID of the node to blame" }
                },
                "required": ["oid"]
            }
        }),
        json!({
            "name": "memory_branch",
            "description": "With `name`: create refs/spectral/heads/{name} at HEAD. Without `name`: list all spectral branches and their tips. Wraps `git update-ref refs/spectral/heads/{name} refs/spectral/HEAD`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Branch name to create. Omit to list all branches." }
                }
            }
        }),
        json!({
            "name": "memory_checkout",
            "description": "Repoint refs/spectral/HEAD (symref) to refs/spectral/heads/{name}. The MCP server's in-memory state is stale until restart; the response includes a note. Wraps `git symbolic-ref refs/spectral/HEAD refs/spectral/heads/{name}`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Existing branch name to switch to" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "memory_thread",
            "description": "Walk a topic note thread chronologically. Tries refs/spectral/notes/topics/{topic}, falls back to refs/spectral/notes/{topic}. Built-in topics: hot-paths, pressure, ticks, wal-folded.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": { "type": "string", "description": "Topic name, e.g. 'hot-paths' or a custom topic under notes/topics/" }
                },
                "required": ["topic"]
            }
        }),
        json!({
            "name": "memory_cherrypick",
            "description": "Replay an existing same-repo commit's tree changes onto the current refs/spectral/HEAD. Cross-repo cherry-pick is not yet supported. The MCP server's in-memory state is stale until restart.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "commit_oid": { "type": "string", "description": "Hex OID of the commit to cherry-pick" }
                },
                "required": ["commit_oid"]
            }
        }),
        json!({
            "name": "graph_query",
            "description": "Execute a pipe-forward graph query. Syntax: `find <type> [|> where <field> <op> <value>] [|> sort by <field> [desc]] [|> limit <n>] [|> count]`. Sources: find, near, hot. Transforms: where, walk, sort, limit. Terminals: count, loss. Every query returns ShannonLoss — bits of information filtered out.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Pipe-forward query string, e.g. 'find eigenboard |> where fiedler > 0.04 |> sort by fiedler desc |> limit 5'" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "memory_gestalt",
            "description": "Traversal<ConceptGraph, Line<@gestalt/memory>> — query the memory graph and annotate results with named lenses. Each lens is a pipe-forward mirror query. Returns a gestalt document.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query":  { "type": "string", "description": "Source query: 'find observation |> where fiedler > 0.04'" },
                    "lenses": { "type": "object", "description": "Named lens map: { \"blame\": \"find blame |> ...\", \"summary\": \"...\" }" }
                },
                "required": ["query"]
            }
        }),
    ]
}

/// Generate tool definitions from scanned grammar actions.
pub fn grammar_tool_definitions(actions: &[GrammarAction]) -> Vec<Value> {
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
                        "content": { "type": "string", "description": "Content for this action" }
                    },
                    "required": ["content"]
                }
            })
        })
        .collect()
}

/// All tool definitions: grammar-scanned + built-in.
pub fn tool_definitions(actions: &[GrammarAction]) -> Value {
    let mut tools = grammar_tool_definitions(actions);
    tools.extend(builtin_tool_definitions());
    Value::Array(tools)
}

/// MCP resource definitions.
pub fn resource_definitions() -> Value {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_tools_include_memory_ops() {
        let tools = builtin_tool_definitions();
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();

        assert!(names.contains(&"memory_store"));
        assert!(names.contains(&"memory_recall"));
        assert!(names.contains(&"memory_crystallize"));
        assert!(names.contains(&"memory_status"));
        assert!(names.contains(&"spectral_loss"));
        assert!(names.contains(&"gestalt_detect"));
        assert!(names.contains(&"graph_query"));
        // Git-native optics
        assert!(names.contains(&"memory_diff"));
        assert!(names.contains(&"memory_blame"));
        assert!(names.contains(&"memory_branch"));
        assert!(names.contains(&"memory_checkout"));
        assert!(names.contains(&"memory_thread"));
        assert!(names.contains(&"memory_cherrypick"));
        assert!(names.contains(&"memory_gestalt"));
    }

    #[test]
    fn grammar_tool_names_use_double_underscore() {
        let actions = vec![GrammarAction {
            grammar_name: "reed".to_string(),
            action_name: "observe".to_string(),
        }];
        let tools = grammar_tool_definitions(&actions);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "reed__observe");
    }

    #[test]
    fn resource_definitions_include_memory() {
        let resources = resource_definitions();
        let arr = resources.as_array().unwrap();
        assert!(arr.iter().any(|r| r["uri"] == "memory://status"));
        assert!(arr.iter().any(|r| r["uri"] == "memory://context"));
    }
}
