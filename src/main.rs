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
        eprintln!("five operations:");
        eprintln!("  spectral focus <path>        observe any structure");
        eprintln!("  spectral project <path>      filter by what matters");
        eprintln!("  spectral split <path>        explore what's connected");
        eprintln!("  spectral zoom <path>         transform one thing");
        eprintln!("  spectral refract <path>      settle. done. crystal.");
        eprintln!();
        eprintln!("session:");
        eprintln!("  spectral init                start a spectral session");
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

    match args[1].as_str() {
        // Five operations
        "focus" | "project" | "split" | "zoom" | "refract" => {
            optic_cmd(&args[1], &args[2..]);
        }

        // Session commands
        "init" => {
            match session::Session::init(Path::new(".")) {
                Ok(_) => process::exit(0),
                Err(e) => {
                    eprintln!("spectral init: {}", e);
                    process::exit(1);
                }
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
                    eprintln!("spectral log: no .spectral directory found (run spectral init)");
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
