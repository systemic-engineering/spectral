//! spectral memory — lens CLI for agent memory.
//!
//! Two graphs:
//! - User graph: ~/.spectral/ — preferences, feedback, patterns across projects
//! - Project graph: .spectral/ — files, functions, decisions, observations per-project
//!
//! Operations: store, recall, crystallize, export, ingest, status.

use std::path::Path;
use std::process;

use lens::export::ExportFormat;
use lens::filter::GrammarFilter;
use lens::types::{Distance, NodeData, NodeType};
use lens::Lens;
use prism::{Oid, Recovery};

/// Open the user lens (~/.spectral/).
pub fn open_user_lens() -> Option<Lens> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let path = format!("{}/.spectral", home);
    let db_path = Path::new(&path);

    if !db_path.exists() {
        std::fs::create_dir_all(db_path).ok()?;
    }

    let filter = user_filter();
    Lens::open(db_path, filter, "user", 1e-6, 50_000_000).ok() // TODO: make configurable via .spec
}

/// Open the project lens (./.spectral/ relative to project_path).
pub fn open_project_lens(project_path: &str) -> Option<Lens> {
    let path = format!("{}/.spectral", project_path);
    let db_path = Path::new(&path);

    if !db_path.exists() {
        std::fs::create_dir_all(db_path).ok()?;
    }

    let filter = project_filter();
    Lens::open(db_path, filter, "project", 1e-6, 50_000_000).ok() // TODO: make configurable via .spec
}

fn user_filter() -> GrammarFilter {
    GrammarFilter::new("user")
        .allow_type("preference")
        .allow_type("feedback")
        .allow_type("reference")
        .allow_type("pattern")
        .allow_type("fact")
        .allow_type("observation")
}

fn project_filter() -> GrammarFilter {
    GrammarFilter::new("project")
        .allow_type("file")
        .allow_type("function")
        .allow_type("decision")
        .allow_type("observation")
        .allow_type("test")
        .allow_type("pattern")
}

/// Store a node in the project graph.
pub fn store(args: &[String]) {
    if args.len() < 2 {
        eprintln!("usage: spectral memory store <type> <content>");
        process::exit(1);
    }
    let type_name = &args[0];
    let content = args[1..].join(" ");

    let lens = open_project_lens(".").unwrap_or_else(|| {
        eprintln!("spectral memory: failed to open project graph");
        process::exit(1);
    });

    let node_type = NodeType::new_unchecked(type_name);
    let data = NodeData::from_str(&content);
    let beam = lens.store(node_type, data);
    lens.flush();

    if beam.is_lossless() {
        println!("{}", beam.result);
    } else {
        let reason = match &beam.recovered {
            Some(Recovery::Failed { reason }) => reason.as_str(),
            _ => "unknown error",
        };
        eprintln!("spectral memory store: {}", reason);
        process::exit(1);
    }
}

/// Recall nodes near an OID from the project graph.
pub fn recall(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: spectral memory recall <oid> [--distance 0.5]");
        process::exit(1);
    }
    let query_oid = &args[0];
    let distance = args
        .iter()
        .position(|a| a == "--distance")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.5);

    let lens = open_project_lens(".").unwrap_or_else(|| {
        eprintln!("spectral memory: failed to open project graph");
        process::exit(1);
    });

    let beam = lens.recall(Oid::new(query_oid), Distance::new(distance));
    let oids = &beam.result;

    if oids.is_empty() {
        eprintln!("  (no results within distance {})", distance);
    } else {
        for oid in oids {
            // Read each node to get its data
            let read_beam = lens.read(oid.clone());
            match read_beam.result {
                Some(data) => {
                    let text = String::from_utf8_lossy(data.as_bytes());
                    println!("  {} {}", oid, text);
                }
                None => {
                    println!("  {}", oid);
                }
            }
        }
    }
}

/// Crystallize a node by OID.
pub fn crystallize(args: &[String]) {
    if args.is_empty() {
        eprintln!("usage: spectral memory crystallize <oid>");
        process::exit(1);
    }
    let oid = &args[0];

    let lens = open_project_lens(".").unwrap_or_else(|| {
        eprintln!("spectral memory: failed to open project graph");
        process::exit(1);
    });

    let beam = lens.crystallize(Oid::new(oid));
    lens.flush();
    if beam.is_lossless() {
        eprintln!("crystallized: {}", oid);
    } else {
        let reason = match &beam.recovered {
            Some(Recovery::Failed { reason }) => reason.as_str(),
            _ => "unknown error",
        };
        eprintln!("spectral memory crystallize: {}", reason);
        process::exit(1);
    }
}

/// Report graph stats for user + project graphs.
pub fn status() {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());

    eprintln!("spectral memory status");

    // User graph
    let user_path = format!("{}/.spectral", home);
    if Path::new(&user_path).exists() {
        if let Some(lens) = open_user_lens() {
            let (nodes, edges) = lens.graph_stats();
            eprintln!("  user:    {} nodes, {} edges ({})", nodes, edges, user_path);
        } else {
            eprintln!("  user:    failed to open ({})", user_path);
        }
    } else {
        eprintln!("  user:    not initialized ({})", user_path);
    }

    // Project graph
    if Path::new(".spectral").exists() {
        if let Some(lens) = open_project_lens(".") {
            let (nodes, edges) = lens.graph_stats();
            eprintln!("  project: {} nodes, {} edges (.spectral/)", nodes, edges);
        } else {
            eprintln!("  project: failed to open (.spectral/)");
        }
    } else {
        eprintln!("  project: not initialized (.spectral/)");
    }
}

/// Export project graph to a directory.
pub fn export(args: &[String]) {
    let dir = args
        .iter()
        .position(|a| a == "--dir")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or(".");

    let lens = open_project_lens(".").unwrap_or_else(|| {
        eprintln!("spectral memory: failed to open project graph");
        process::exit(1);
    });

    let beam = lens.export_to(Path::new(dir), ExportFormat::Markdown);
    if beam.is_lossless() {
        eprintln!("exported to {}", dir);
    } else {
        let reason = match &beam.recovered {
            Some(Recovery::Failed { reason }) => reason.as_str(),
            _ => "partial export",
        };
        eprintln!("exported with loss: {}", reason);
    }
}

/// Ingest markdown memory files from a directory into the project graph.
pub fn ingest(args: &[String]) {
    let dir = args
        .iter()
        .position(|a| a == "--dir")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or(".");

    let lens = open_project_lens(".").unwrap_or_else(|| {
        eprintln!("spectral memory: failed to open project graph");
        process::exit(1);
    });

    let beam = lens.ingest_from(Path::new(dir));
    lens.flush();
    let count = beam.result.len();
    if beam.is_lossless() {
        eprintln!("ingested {} nodes from {}", count, dir);
    } else {
        let reason = match &beam.recovered {
            Some(Recovery::Failed { reason }) => reason.as_str(),
            _ => "partial ingest",
        };
        eprintln!("ingested {} nodes with loss: {}", count, reason);
    }
}
