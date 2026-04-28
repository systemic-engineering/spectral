//! spectral — git for graphs.
//!
//! One binary. Five operations. Everything settles.
//!
//! ```
//! spectral focus .             observe any structure
//! spectral project .           filter by what matters
//! spectral split .             explore what's connected
//! spectral zoom .              transform one thing
//! spectral refract .           settle. done. crystal.
//!
//! spectral init                start a spectral session
//! spectral tick                advance the clock
//! spectral tock                settle the graph
//! spectral shatter             break apart a composite
//!
//! spectral diff                compare two states
//! spectral log                 show tick history
//! spectral blame               trace a node's lineage
//!
//! spectral mirror <cmd>        compiler operations
//! spectral memory <cmd>        lens memory operations
//! spectral serve               MCP server
//! ```

mod diff;
mod log;
mod memory;
mod observe;
mod refs;
mod serve;
mod session;

use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("spectral — git for graphs");
        eprintln!();
        eprintln!("optics (each subcommand IS an optic — add --json for machine output):");
        eprintln!("  spectral status [path]       Lens    — nodes, edges, tension, loss");
        eprintln!("  spectral savings [path]      Lens    — token savings, cache efficiency");
        eprintln!("  spectral loss [path]         Fold    — per-file loss, sorted descending");
        eprintln!("  spectral peers [path]        Traversal — known peers");
        eprintln!("  spectral crystal [path]      Prism   — crystallized nodes");
        eprintln!("  spectral benchmark [path]    Lens    — hook latencies, SLO status");
        eprintln!("  spectral index [path]        Traversal  — gestalt → edges → cascade → crystallize");
    eprintln!("  spectral join @ctx --add .   AffineTraversal — TUI session");
        eprintln!();
        eprintln!("five operations:");
        eprintln!("  spectral focus <path>        observe any structure");
        eprintln!("  spectral project <path>      filter by what matters");
        eprintln!("  spectral split <path>        explore what's connected");
        eprintln!("  spectral zoom <path>         transform one thing");
        eprintln!("  spectral refract <path>      settle. done. crystal.");
        eprintln!();
        eprintln!("session:");
        eprintln!("  spectral init                start a spectral session");
        eprintln!("  spectral repl                shard> prompt");
        eprintln!("  spectral tick                advance the clock");
        eprintln!("  spectral tock                settle the graph");
        eprintln!("  spectral shatter             break apart a composite");
        eprintln!();
        eprintln!("navigation:");
        eprintln!("  spectral diff                compare two states");
        eprintln!("  spectral log                 show tick history");
        eprintln!("  spectral blame               trace a node's lineage");
        eprintln!();
        eprintln!("tools:");
        eprintln!("  spectral mirror <cmd>        compiler");
        eprintln!("  spectral memory <cmd>        agent memory");
        eprintln!("  spectral serve [--project .]  MCP server");
        process::exit(1);
    }

    let json_flag = args.iter().any(|a| a == "--json");

    match args[1].as_str() {
        // Optic subcommands — typed by optic, zero inference cost
        "status" => {
            let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let view = spectral::apache2::views::StatusView::from_session(Path::new(path));
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&view).unwrap());
            } else {
                println!("{}", view.format());
            }
        }

        "savings" => {
            let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let view = spectral::apache2::views::SavingsView::from_session(Path::new(path));
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&view).unwrap());
            } else {
                println!("{}", view.format());
            }
        }

        "loss" => {
            let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let view = spectral::apache2::views::LossView::from_session(Path::new(path));
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&view).unwrap());
            } else {
                println!("{}", view.format());
            }
        }

        "peers" => {
            let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let view = spectral::apache2::views::PeersView::from_session(Path::new(path));
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&view).unwrap());
            } else {
                println!("{}", view.format());
            }
        }

        "crystal" => {
            let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let view = spectral::apache2::views::CrystalView::from_session(Path::new(path));
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&view).unwrap());
            } else {
                println!("{}", view.format());
            }
        }

        "benchmark" => {
            let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let view = spectral::apache2::views::BenchmarkView::from_session(Path::new(path));
            if json_flag {
                println!("{}", serde_json::to_string_pretty(&view).unwrap());
            } else {
                println!("{}", view.format());
            }
        }

        // Five operations
        "focus" | "project" | "split" | "zoom" | "refract" => {
            optic_cmd(&args[1], &args[2..]);
        }

        // Session commands
        "init" => {
            let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let target = Path::new(path);

            // Phase 1: identity observation via .mirror files + gestalt auto-detection
            let (snapshot, eigenvalue_profile) = match spectral::apache2::init::init_identity(target) {
                terni::Imperfect::Success(result) => {
                    eprintln!("spectral init: {} grammars compiled", result.mirror_files_found);
                    eprintln!("  bias chain: {}", result.bias_chain.ordering().join(" => "));
                    eprintln!("  fast oid:   {}", result.snapshot.fast_oid);
                    eprintln!("  full oid:   {}", result.snapshot.full_oid);
                    eprintln!("  state:      {} bytes", result.snapshot.state_bytes);
                    eprintln!("  holonomy: 0.000 (crystal)");
                    // Gestalt auto-detection enrichment
                    if result.gestalt_files_detected > 0 {
                        eprintln!("  gestalt:    {} files (md:{} code:{} config:{} asset:{} other:{})",
                            result.gestalt_files_detected,
                            result.gestalt_breakdown.markdown,
                            result.gestalt_breakdown.code,
                            result.gestalt_breakdown.config,
                            result.gestalt_breakdown.asset,
                            result.gestalt_breakdown.other,
                        );
                        if let Some(ref graph) = result.concept_graph {
                            eprintln!("  graph:      {} nodes, {} edges",
                                graph.nodes.len(), graph.edges.len());
                        }
                        if let Some(ref profile) = result.eigenvalue_profile {
                            eprintln!("  eigenvalue: fiedler={:.4}", profile.fiedler_value());
                        }
                    }
                    (Some(result.snapshot), result.eigenvalue_profile)
                }
                terni::Imperfect::Partial(result, _loss) => {
                    // Gestalt-only identity (no .mirror files)
                    eprintln!("spectral init: gestalt auto-detection ({} files)", result.gestalt_files_detected);
                    eprintln!("  breakdown:  md:{} code:{} config:{} asset:{} other:{}",
                        result.gestalt_breakdown.markdown,
                        result.gestalt_breakdown.code,
                        result.gestalt_breakdown.config,
                        result.gestalt_breakdown.asset,
                        result.gestalt_breakdown.other,
                    );
                    if let Some(ref graph) = result.concept_graph {
                        eprintln!("  graph:      {} nodes, {} edges",
                            graph.nodes.len(), graph.edges.len());
                    }
                    if let Some(ref profile) = result.eigenvalue_profile {
                        eprintln!("  eigenvalue: fiedler={:.4}", profile.fiedler_value());
                    }
                    eprintln!("  fast oid:   {}", result.snapshot.fast_oid);
                    eprintln!("  full oid:   {}", result.snapshot.full_oid);
                    (Some(result.snapshot), result.eigenvalue_profile)
                }
                terni::Imperfect::Failure(msg, _) => {
                    eprintln!("spectral init: {}", msg);
                    (None, None)
                }
            };

            // Phase 2: session directory (.git/spectral/)
            match session::Session::init(target) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("spectral init: {}", e);
                    process::exit(1);
                }
            }

            // Phase 3: write two-tier snapshot + eigenvalue profile to .git/spectral/
            if let Some(snap) = snapshot {
                let spectral_dir = target.join(".git").join("spectral");
                // Fast hash = session anchor. Updates on every spectral operation.
                if let Err(e) = std::fs::write(spectral_dir.join("fast_oid"), snap.fast_oid.as_str()) {
                    eprintln!("spectral: failed to write fast_oid: {}", e);
                }
                // Full hash = identity anchor. Updates on crystallization events.
                if let Err(e) = std::fs::write(spectral_dir.join("full_oid"), snap.full_oid.as_str()) {
                    eprintln!("spectral: failed to write full_oid: {}", e);
                }
                // Eigenvalue profile = spectral fingerprint.
                if let Some(ref profile) = eigenvalue_profile {
                    let profile_str = profile.values
                        .iter()
                        .map(|v| format!("{:.8}", v))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if let Err(e) = std::fs::write(spectral_dir.join("eigenvalue_profile"), &profile_str) {
                        eprintln!("spectral: failed to write eigenvalue_profile: {}", e);
                    }
                }
            }

            // Create .git/mirror/ for crystal storage
            let mirror_dir = target.join(".git/mirror");
            let git_dir = target.join(".git");
            if mirror_dir.exists() {
                eprintln!("spectral: .git/mirror/ already exists");
            } else if git_dir.exists() {
                match std::fs::create_dir_all(&mirror_dir) {
                    Ok(_) => eprintln!("spectral: initialized .git/mirror/"),
                    Err(e) => eprintln!("spectral: failed to create .git/mirror/: {}", e),
                }
            } else {
                eprintln!("spectral: no .git directory (not a git repo — skipping .git/mirror/)");
            }
            process::exit(0);
        }

        // REPL — shard> prompt
        "repl" => {
            use std::io::{self, Write, BufRead};
            let stdin = io::stdin();
            loop {
                eprint!("shard> ");
                io::stderr().flush().unwrap();
                let mut line = String::new();
                if stdin.lock().read_line(&mut line).unwrap() == 0 { break; }
                let trimmed = line.trim();
                if trimmed == "exit" || trimmed == "quit" { break; }
                if trimmed.is_empty() { continue; }
                // dispatch through mirror
                println!("  (not yet wired)");
            }
        }

        // Session commands — not yet implemented
        "tick" | "tock" | "shatter" => {
            eprintln!("spectral {}: not yet implemented", args[1]);
            process::exit(1);
        }

        // Navigation commands
        "diff" => {
            if args.len() < 4 {
                eprintln!("usage: spectral diff <ref-a> <ref-b>");
                process::exit(1);
            }
            // For now, print stub — full integration with session state comes later
            eprintln!("spectral diff {} {}", args[2], args[3]);
            eprintln!("  (full diff requires session state — run spectral init first)");
        }

        "log" => {
            let oneline = args.iter().any(|a| a == "--oneline");
            match session::Session::find(Path::new(".")) {
                Some(session) => {
                    let entries = log::read_log(&session);
                    let output = log::format_log(&entries, oneline);
                    eprint!("{}", output);
                }
                None => {
                    eprintln!("spectral log: no .git/spectral directory found (run spectral init)");
                    process::exit(1);
                }
            }
        }

        "blame" => {
            eprintln!("spectral blame: not yet implemented");
            process::exit(1);
        }

        // Tool subcommands — delegate to binaries
        "mirror" => delegate("mirror", &args[2..]),

        // Memory — lens CLI
        "memory" => memory_cmd(&args[2..]),

        // Index — Traversal<File, Crystal>: gestalt → edges → cascade → crystallize
        // Full pipeline runs via MCP (actor state required for cascade + crystallize).
        // CLI: gestalt stage only. Use mcp__spectral__spectral_index for full pipeline.
        "index" => {
            let path = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            let target = std::path::Path::new(path);
            let cached = spectral::apache2::graph_cache::load_or_build(target);
            let graph = &cached.graph;
            let profile = &cached.profile;
            let breakdown = &cached.breakdown;
            println!("indexed: {}", path);
            println!(
                "  files:   {} (md:{} code:{} config:{} mirror:{})",
                breakdown.total(), breakdown.markdown, breakdown.code,
                breakdown.config, breakdown.mirror
            );
            println!("  graph:   {} nodes, {} edges", graph.nodes.len(), graph.edges.len());
            if !profile.is_dark() {
                println!("  fiedler: {:.4}", profile.fiedler_value());
                println!("  oid:     {}", profile.oid());
            } else {
                println!("  fiedler: dark (no connectivity)");
            }
            if cached.from_cache {
                println!("  source:  cached (.git/spectral/contexts/graph.json)");
            } else {
                println!("  source:  computed (gestalt scan)");
            }
            // Cascade + crystallize run via MCP (persistent actor state required)
            println!("  cascade: via mcp__spectral__spectral_index");
            println!("  crystals: via mcp__spectral__spectral_index");
        }

        // MCP server
        "serve" => {
            let project = args
                .iter()
                .position(|a| a == "--project")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.as_str())
                .unwrap_or(".");
            serve::serve(project);
        }

        // Join — TUI eigenboard session (requires --features sel)
        "join" => {
            #[cfg(feature = "sel")]
            {
                let second = args.get(2).map(|s| s.as_str());
                match second {
                    // spectral join <path> — peer join (bare path, no @ prefix)
                    Some(p) if !p.starts_with('@') && !p.starts_with('-') => {
                        let path = std::path::Path::new(p);
                        match spectral::sel::join::join_peer(path) {
                            Ok(msg) => eprintln!("{}", msg),
                            Err(e) => {
                                eprintln!("spectral join: {}", e);
                                process::exit(1);
                            }
                        }
                    }
                    // spectral join @context --add /path ... — TUI session
                    Some(p) if p.starts_with('@') => {
                        let context_name = p.trim_start_matches('@');
                        let mut add_paths: Vec<&str> = Vec::new();
                        let mut i = 3;
                        while i < args.len() {
                            if args[i] == "--add" {
                                if let Some(path) = args.get(i + 1) {
                                    add_paths.push(path.as_str());
                                    i += 2;
                                } else {
                                    eprintln!("spectral join: --add requires a path");
                                    process::exit(1);
                                }
                            } else {
                                i += 1;
                            }
                        }
                        if let Err(e) = spectral::sel::tui::run_tui(context_name, &add_paths) {
                            eprintln!("spectral join: {}", e);
                            process::exit(1);
                        }
                    }
                    // spectral join (no args) — REPL
                    _ => {
                        if let Err(e) = spectral::sel::join::join(std::path::Path::new(".")) {
                            eprintln!("spectral join: {}", e);
                            process::exit(1);
                        }
                    }
                }
            }
            #[cfg(not(feature = "sel"))]
            {
                eprintln!("spectral join: requires --features sel");
                process::exit(1);
            }
        }

        // Observe — internal command. Fast (<5ms). No actor system.
        // Reads JSON from stdin: {tool_name, tool_input, tool_response}
        // Writes to .git/spectral/inbox/{nanos}-{pid}.json — silent on success.
        "observe" => {
            use std::io::Read;
            let mut raw = String::new();
            let _ = std::io::stdin().read_to_string(&mut raw);

            // Bail silently on empty or invalid JSON — don't write garbage to inbox.
            if raw.trim().is_empty() {
                return;
            }
            let v: serde_json::Value = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(_) => return,
            };

            let tool_name = match v.get("tool_name")
                .or_else(|| v.get("tool"))
                .and_then(|x| x.as_str())
            {
                Some(name) => name.to_string(),
                None => return,
            };

            let input_raw = v.get("tool_input").or_else(|| v.get("input"));
            let input_summary = input_raw
                .map(|x| {
                    let s = x.to_string();
                    s.chars().take(500).collect::<String>()
                })
                .unwrap_or_default();

            let output_raw = v.get("tool_response").or_else(|| v.get("output"));
            let output_summary = output_raw
                .map(|x| {
                    let s = x.to_string();
                    s.chars().take(500).collect::<String>()
                })
                .unwrap_or_default();

            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            if let Err(e) = observe::write_observation(&cwd, &tool_name, &input_summary, &output_summary) {
                eprintln!("{}", e);
            }
        }

        other => {
            eprintln!("spectral: unknown command '{}'", other);
            process::exit(1);
        }
    }
}

/// Delegate to an external binary.
fn delegate(binary: &str, args: &[String]) {
    let status = std::process::Command::new(binary)
        .args(args)
        .status();
    match status {
        Ok(s) => process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("spectral: failed to run '{}': {}", binary, e);
            process::exit(1);
        }
    }
}

/// Five operations — focus, project, split, zoom, refract.
/// Each parses .mirror/.conv source into a content-addressed AST and prints the graph.
fn optic_cmd(op: &str, args: &[String]) {
    use mirror::parse::Parse;
    use mirror::Vector;

    let path = args.first().map(|s| s.as_str()).unwrap_or(".");

    // If path is a file, parse it as .mirror grammar
    let source = if std::path::Path::new(path).is_file() {
        std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("spectral {}: {}: {}", op, path, e);
            process::exit(1);
        })
    } else {
        // Directory: scan for all .mirror/.conv files
        eprintln!("spectral {} on directory: scanning {}", op, path);
        let mut combined = String::new();
        if let Ok(entries) = std::fs::read_dir(path) {
            let mut paths: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let p = e.path();
                    p.extension()
                        .and_then(|x| x.to_str())
                        .map_or(false, |ext| ext == "mirror" || ext == "conv")
                })
                .collect();
            paths.sort_by_key(|e| e.path());
            for entry in paths {
                if let Ok(s) = std::fs::read_to_string(entry.path()) {
                    combined.push_str(&s);
                    combined.push('\n');
                }
            }
        }
        combined
    };

    if source.is_empty() {
        eprintln!("spectral {}: no .mirror or .conv files in {}", op, path);
        process::exit(1);
    }

    // Parse
    let ast = match Parse.trace(source).into_result() {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!("spectral {}: parse error: {}", op, e);
            process::exit(1);
        }
    };

    let node_count = ast.children().len();
    eprintln!("spectral {}: {} nodes from {}", op, node_count, path);

    // Print the graph as node list
    for child in ast.children() {
        println!("  {}:{}", child.data().name, child.data().value);
    }
}

/// Memory subcommands — store, recall, crystallize, export, ingest, status.
fn memory_cmd(args: &[String]) {
    if args.is_empty() {
        eprintln!("spectral memory — agent memory via lens");
        eprintln!();
        eprintln!("  spectral memory store <type> <content>");
        eprintln!("  spectral memory recall <oid> [--distance 0.5]");
        eprintln!("  spectral memory crystallize <oid>");
        eprintln!("  spectral memory export [--dir .]");
        eprintln!("  spectral memory ingest [--dir .]");
        eprintln!("  spectral memory status");
        process::exit(1);
    }

    match args[0].as_str() {
        "store" => memory::store(&args[1..]),
        "recall" => memory::recall(&args[1..]),
        "crystallize" => memory::crystallize(&args[1..]),
        "export" => memory::export(&args[1..]),
        "ingest" => memory::ingest(&args[1..]),
        "status" => memory::status(),
        other => {
            eprintln!("spectral memory: unknown command '{}'", other);
            process::exit(1);
        }
    }
}
