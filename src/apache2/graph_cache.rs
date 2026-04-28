//! Graph cache — read/write ConceptGraph from `.git/spectral/contexts/`.
//!
//! The cache is keyed by a directory hash: a fast fingerprint of the file
//! listing + modification times. If the dir_hash matches, the cached graph
//! is fresh. If not, recompute and write.
//!
//! This is the convergence point: CLI and MCP both read/write the same
//! cache. First run is slow (gestalt scan). Second run is instant (JSON read).

use std::path::Path;

use gestalt::eigenvalue::EigenvalueProfile;
use gestalt::graph::ConceptGraph;

/// Result of loading or computing a concept graph.
pub struct CachedGraph {
    pub graph: ConceptGraph,
    pub profile: EigenvalueProfile,
    pub breakdown: gestalt::detect::GestaltBreakdown,
    /// True if the graph was loaded from cache (fast path).
    pub from_cache: bool,
}

/// Compute a fast directory fingerprint: sorted file paths + sizes.
///
/// Uses file sizes instead of mtimes for determinism in tests.
/// Content changes always change size or file count.
pub fn dir_hash(path: &Path) -> String {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut entries: Vec<(String, u64)> = Vec::new();
    collect_dir_entries(path, &mut entries);
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = DefaultHasher::new();
    for (name, size) in &entries {
        name.hash(&mut hasher);
        size.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish())
}

fn collect_dir_entries(path: &Path, entries: &mut Vec<(String, u64)>) {
    let read_dir = match std::fs::read_dir(path) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    for entry in read_dir.flatten() {
        let file_path = entry.path();
        // Skip .git directory
        if file_path
            .file_name()
            .map_or(false, |n| n == ".git" || n == ".spectral")
        {
            continue;
        }
        if file_path.is_dir() {
            collect_dir_entries(&file_path, entries);
        } else if let Ok(meta) = file_path.metadata() {
            let rel = file_path
                .strip_prefix(path)
                .unwrap_or(&file_path)
                .to_string_lossy()
                .to_string();
            entries.push((rel, meta.len()));
        }
    }
}

/// Path to the cached graph JSON file.
fn graph_json_path(path: &Path) -> std::path::PathBuf {
    path.join(".git/spectral/contexts/graph.json")
}

/// Path to the cached profile JSON file.
fn profile_json_path(path: &Path) -> std::path::PathBuf {
    path.join(".git/spectral/contexts/profile.json")
}

/// Try to load a cached graph. Returns None if cache is missing or stale.
pub fn load_cached_graph(path: &Path) -> Option<CachedGraph> {
    let graph_path = graph_json_path(path);
    let profile_path = profile_json_path(path);

    let graph_content = std::fs::read_to_string(&graph_path).ok()?;
    let graph_val: serde_json::Value = serde_json::from_str(&graph_content).ok()?;

    // Check dir_hash freshness
    let stored_hash = graph_val.get("dir_hash")?.as_str()?;
    let current_hash = dir_hash(path);
    if stored_hash != current_hash {
        return None; // stale
    }

    // Reconstruct ConceptGraph from JSON
    let graph = reconstruct_graph(&graph_val)?;

    // Load profile
    let profile = if profile_path.exists() {
        let profile_content = std::fs::read_to_string(&profile_path).ok()?;
        let profile_val: serde_json::Value = serde_json::from_str(&profile_content).ok()?;
        reconstruct_profile(&profile_val)?
    } else {
        gestalt::eigenvalue::eigenvalue_profile(&graph)
    };

    // Reconstruct breakdown from graph metadata
    let breakdown = reconstruct_breakdown(&graph_val);

    Some(CachedGraph {
        graph,
        profile,
        breakdown,
        from_cache: true,
    })
}

/// Build the concept graph from scratch and write to cache.
pub fn build_and_cache(path: &Path) -> CachedGraph {
    let (graph, _files, breakdown) = gestalt::graph::build_concept_graph(path);
    let profile = gestalt::eigenvalue::eigenvalue_profile(&graph);

    // Write to cache if .git/spectral/ exists
    let _ = write_graph_cache(path, &graph, &profile, &breakdown);

    CachedGraph {
        graph,
        profile,
        breakdown,
        from_cache: false,
    }
}

/// Load from cache if fresh, otherwise build and cache.
pub fn load_or_build(path: &Path) -> CachedGraph {
    if let Some(cached) = load_cached_graph(path) {
        return cached;
    }
    build_and_cache(path)
}

/// Write graph + profile + dir_hash to `.git/spectral/contexts/`.
pub fn write_graph_cache(
    path: &Path,
    graph: &ConceptGraph,
    profile: &EigenvalueProfile,
    breakdown: &gestalt::detect::GestaltBreakdown,
) -> Result<(), String> {
    let contexts_dir = path.join(".git/spectral/contexts");
    std::fs::create_dir_all(&contexts_dir)
        .map_err(|e| format!("failed to create contexts dir: {}", e))?;

    let current_hash = dir_hash(path);

    // Graph JSON with dir_hash for staleness check
    let nodes_json: Vec<serde_json::Value> = graph
        .nodes
        .iter()
        .map(|n| {
            serde_json::json!({
                "name": n.name(),
                "oid": n.oid().to_string(),
                "file_count": n.file_count(),
            })
        })
        .collect();

    let edges_json: Vec<serde_json::Value> = graph
        .edges
        .iter()
        .map(|e| {
            let (a, b) = e.indices();
            let edge_type = match e {
                gestalt::graph::GraphEdge::Contains { .. } => "contains",
                gestalt::graph::GraphEdge::SimilarContent { .. } => "similar_content",
                gestalt::graph::GraphEdge::CrossRef { .. } => "cross_ref",
            };
            serde_json::json!({
                "from": a,
                "to": b,
                "weight": e.weight(),
                "type": edge_type,
            })
        })
        .collect();

    let graph_json = serde_json::json!({
        "dir_hash": current_hash,
        "node_count": graph.nodes.len(),
        "edge_count": graph.edges.len(),
        "nodes": nodes_json,
        "edges": edges_json,
        "breakdown": {
            "markdown": breakdown.markdown,
            "code": breakdown.code,
            "config": breakdown.config,
            "asset": breakdown.asset,
            "mirror": breakdown.mirror,
            "gestalt_native": breakdown.gestalt_native,
            "other": breakdown.other,
        },
    });

    std::fs::write(
        contexts_dir.join("graph.json"),
        serde_json::to_string_pretty(&graph_json).unwrap_or_default(),
    )
    .map_err(|e| format!("failed to write graph.json: {}", e))?;

    // Profile JSON
    if !profile.is_dark() {
        let profile_json = serde_json::json!({
            "fiedler": profile.fiedler_value(),
            "values": profile.values.to_vec(),
            "oid": profile.oid().to_string(),
            "nodes": graph.nodes.len(),
            "edges": graph.edges.len(),
        });
        std::fs::write(
            contexts_dir.join("profile.json"),
            serde_json::to_string_pretty(&profile_json).unwrap_or_default(),
        )
        .map_err(|e| format!("failed to write profile.json: {}", e))?;
    }

    Ok(())
}

/// Reconstruct a ConceptGraph from cached JSON.
fn reconstruct_graph(val: &serde_json::Value) -> Option<ConceptGraph> {
    let nodes_arr = val.get("nodes")?.as_array()?;
    let edges_arr = val.get("edges")?.as_array()?;

    let nodes: Vec<gestalt::graph::GraphNode> = nodes_arr
        .iter()
        .map(|n| {
            let name = n.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let file_count = n.get("file_count").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            gestalt::graph::GraphNode::Directory {
                path: std::path::PathBuf::from(&name),
                name,
                depth: 0,
                file_count,
            }
        })
        .collect();

    let edges: Vec<gestalt::graph::GraphEdge> = edges_arr
        .iter()
        .filter_map(|e| {
            let from = e.get("from")?.as_u64()? as usize;
            let to = e.get("to")?.as_u64()? as usize;
            let weight = e.get("weight")?.as_f64()?;
            let edge_type = e.get("type")?.as_str()?;
            Some(match edge_type {
                "contains" => gestalt::graph::GraphEdge::Contains {
                    parent_idx: from,
                    child_idx: to,
                    weight,
                },
                "similar_content" => gestalt::graph::GraphEdge::SimilarContent {
                    a_idx: from,
                    b_idx: to,
                    weight,
                },
                "cross_ref" => gestalt::graph::GraphEdge::CrossRef {
                    source_idx: from,
                    target_idx: to,
                    weight,
                },
                _ => return None,
            })
        })
        .collect();

    Some(ConceptGraph { nodes, edges })
}

/// Reconstruct an EigenvalueProfile from cached JSON.
fn reconstruct_profile(val: &serde_json::Value) -> Option<EigenvalueProfile> {
    let values_arr = val.get("values")?.as_array()?;
    if values_arr.len() < 16 {
        return None;
    }
    let mut values = [0.0f64; 16];
    for (i, v) in values_arr.iter().take(16).enumerate() {
        values[i] = v.as_f64()?;
    }
    Some(EigenvalueProfile { values })
}

/// Reconstruct GestaltBreakdown from cached JSON.
fn reconstruct_breakdown(val: &serde_json::Value) -> gestalt::detect::GestaltBreakdown {
    let bd = val.get("breakdown");
    gestalt::detect::GestaltBreakdown {
        markdown: bd.and_then(|b| b.get("markdown")).and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        code: bd.and_then(|b| b.get("code")).and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        config: bd.and_then(|b| b.get("config")).and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        asset: bd.and_then(|b| b.get("asset")).and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        mirror: bd.and_then(|b| b.get("mirror")).and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        gestalt_native: bd.and_then(|b| b.get("gestalt_native")).and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        other: bd.and_then(|b| b.get("other")).and_then(|v| v.as_u64()).unwrap_or(0) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_hash_deterministic() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.md"), "# Hello").unwrap();
        std::fs::write(tmp.path().join("b.rs"), "fn main() {}").unwrap();

        let h1 = dir_hash(tmp.path());
        let h2 = dir_hash(tmp.path());
        assert_eq!(h1, h2, "dir_hash must be deterministic");
    }

    #[test]
    fn dir_hash_changes_on_new_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.md"), "# Hello").unwrap();
        let h1 = dir_hash(tmp.path());

        std::fs::write(tmp.path().join("b.rs"), "fn main() {}").unwrap();
        let h2 = dir_hash(tmp.path());
        assert_ne!(h1, h2, "dir_hash must change when files are added");
    }

    #[test]
    fn dir_hash_changes_on_content_change() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.md"), "# Hello").unwrap();
        let h1 = dir_hash(tmp.path());

        std::fs::write(tmp.path().join("a.md"), "# Hello World — longer content").unwrap();
        let h2 = dir_hash(tmp.path());
        assert_ne!(h1, h2, "dir_hash must change when file content changes size");
    }

    #[test]
    fn build_and_cache_writes_graph_json() {
        let tmp = tempfile::tempdir().unwrap();
        // Create .git/spectral/ so cache can write
        std::fs::create_dir_all(tmp.path().join(".git/spectral")).unwrap();
        std::fs::write(tmp.path().join("readme.md"), "# Test\n\nContent here.\n").unwrap();
        let sub = tmp.path().join("src");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("lib.rs"), "pub fn main() {}\n").unwrap();

        let result = build_and_cache(tmp.path());
        assert!(!result.from_cache, "first build should not be from cache");

        let graph_path = tmp.path().join(".git/spectral/contexts/graph.json");
        assert!(graph_path.exists(), "graph.json must be written");

        let content = std::fs::read_to_string(&graph_path).unwrap();
        let val: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(val.get("dir_hash").is_some(), "must contain dir_hash");
        assert!(val.get("nodes").is_some(), "must contain nodes");
        assert!(val.get("breakdown").is_some(), "must contain breakdown");
    }

    #[test]
    fn load_or_build_uses_cache_on_second_call() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git/spectral")).unwrap();
        std::fs::write(tmp.path().join("readme.md"), "# Hello\n").unwrap();

        // First call: builds and caches
        let r1 = load_or_build(tmp.path());
        assert!(!r1.from_cache, "first call should compute from scratch");

        // Second call: should use cache
        let r2 = load_or_build(tmp.path());
        assert!(r2.from_cache, "second call should load from cache");
    }

    #[test]
    fn load_or_build_recomputes_when_stale() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git/spectral")).unwrap();
        std::fs::write(tmp.path().join("readme.md"), "# Hello\n").unwrap();

        // Build and cache
        let r1 = load_or_build(tmp.path());
        assert!(!r1.from_cache);

        // Modify directory — add a file
        std::fs::write(tmp.path().join("new_file.txt"), "changed!").unwrap();

        // Should detect stale cache and recompute
        let r2 = load_or_build(tmp.path());
        assert!(!r2.from_cache, "should recompute after directory change");
    }

    #[test]
    fn load_cached_graph_returns_none_without_cache() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load_cached_graph(tmp.path()).is_none());
    }

    #[test]
    fn cached_graph_preserves_node_count() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git/spectral")).unwrap();
        std::fs::write(tmp.path().join("readme.md"), "# Hello\n").unwrap();
        let sub = tmp.path().join("docs");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("guide.md"), "# Guide\n").unwrap();

        let r1 = build_and_cache(tmp.path());
        let r2 = load_or_build(tmp.path());
        assert!(r2.from_cache);
        assert_eq!(
            r1.graph.nodes.len(),
            r2.graph.nodes.len(),
            "cached graph must preserve node count"
        );
        assert_eq!(
            r1.graph.edges.len(),
            r2.graph.edges.len(),
            "cached graph must preserve edge count"
        );
    }

    #[test]
    fn cached_graph_preserves_breakdown() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git/spectral")).unwrap();
        std::fs::write(tmp.path().join("readme.md"), "# Hello\n").unwrap();
        std::fs::write(tmp.path().join("lib.rs"), "fn main() {}\n").unwrap();

        let r1 = build_and_cache(tmp.path());
        let r2 = load_or_build(tmp.path());
        assert!(r2.from_cache);
        assert_eq!(
            r1.breakdown.markdown, r2.breakdown.markdown,
            "breakdown.markdown must survive cache"
        );
        assert_eq!(
            r1.breakdown.code, r2.breakdown.code,
            "breakdown.code must survive cache"
        );
    }
}
