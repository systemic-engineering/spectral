//! spectral — jq for reality.
//!
//! One binary. Five operations. Everything settles.
//!
//! ```
//! spectral fold .              observe any structure
//! spectral prism .             filter by what matters
//! spectral traversal .         explore what's connected
//! spectral lens .              transform one thing
//! spectral iso .               settle. done. crystal.
//!
//! spectral mirror <cmd>        compiler operations
//! spectral conversation <cmd>  runtime operations
//! spectral db <cmd>            spectral-db operations
//! spectral memory <cmd>        lens memory operations
//! spectral serve               MCP server
//! ```

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("spectral — jq for reality");
        eprintln!();
        eprintln!("five operations:");
        eprintln!("  spectral fold <path>         observe any structure");
        eprintln!("  spectral prism <path>        filter by what matters");
        eprintln!("  spectral traversal <path>    explore what's connected");
        eprintln!("  spectral lens <path>         transform one thing");
        eprintln!("  spectral iso <path>          settle. done. crystal.");
        eprintln!();
        eprintln!("tools:");
        eprintln!("  spectral mirror <cmd>        compiler");
        eprintln!("  spectral conversation <cmd>  runtime");
        eprintln!("  spectral db <cmd>            graph database");
        eprintln!("  spectral memory <cmd>        agent memory");
        eprintln!("  spectral serve [--project .]  MCP server");
        process::exit(1);
    }

    match args[1].as_str() {
        // Five operations — delegate to mirror's abyss
        "fold" | "prism" | "traversal" | "lens" | "iso" => {
            optic_cmd(&args[1], &args[2..]);
        }

        // Tool subcommands — delegate to binaries
        "mirror" => delegate("mirror", &args[2..]),
        "conversation" => delegate("conversation", &args[2..]),
        "db" => delegate("spectral-db", &args[2..]),

        // Memory — lens CLI
        "memory" => memory_cmd(&args[2..]),

        // MCP server
        "serve" => {
            eprintln!("spectral serve: not yet wired (Task 4)");
            process::exit(1);
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

/// Five operations — fold, prism, traversal, lens, iso.
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

/// Memory subcommands — store, recall, crystallize, export, ingest.
fn memory_cmd(args: &[String]) {
    if args.is_empty() {
        eprintln!("spectral memory — agent memory via lens");
        eprintln!();
        eprintln!("  spectral memory store <type> <content>");
        eprintln!("  spectral memory recall <query> [--distance 0.5]");
        eprintln!("  spectral memory crystallize <oid>");
        eprintln!("  spectral memory export [--dir .]");
        eprintln!("  spectral memory ingest [--dir .]");
        eprintln!("  spectral memory status");
        process::exit(1);
    }

    match args[0].as_str() {
        "status" => memory_status(),
        other => {
            eprintln!("spectral memory {}: not yet wired (Task 3)", other);
            process::exit(1);
        }
    }
}

/// Memory status — open both graphs, report stats.
fn memory_status() {
    let home = dirs_or_home();
    let user_db_path = format!("{}/.spectral", home);
    let project_db_path = ".spectral";

    eprintln!("spectral memory status");
    eprintln!("  user graph:    {}", user_db_path);
    eprintln!("  project graph: {}", project_db_path);

    // Check if dirs exist
    let user_exists = std::path::Path::new(&user_db_path).exists();
    let project_exists = std::path::Path::new(project_db_path).exists();
    eprintln!(
        "  user:    {}",
        if user_exists { "exists" } else { "not initialized" }
    );
    eprintln!(
        "  project: {}",
        if project_exists {
            "exists"
        } else {
            "not initialized"
        }
    );
}

fn dirs_or_home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
}
