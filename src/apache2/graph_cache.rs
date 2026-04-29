//! Graph cache — projects the concept graph from `refs/spectral/HEAD` (the
//! git tree-of-trees written by spectral-db's `settle()` flow).
//!
//! Phase 3 of the git-native migration: we no longer maintain a parallel
//! `.git/spectral/contexts/graph.json` cache. The git OID IS the cache key.
//! The single source of truth is the spectral-db commit tree.
//!
//! Three paths:
//! 1. **Git path:** `refs/spectral/HEAD` (or legacy `refs/spectral/head`)
//!    exists → walk the tree, count nodes/edges, return.
//! 2. **Fallback path:** no spectral ref → run the gestalt concept-graph
//!    scan exactly like before (so a fresh project still gets sensible
//!    output before any spectral_index has run).
//! 3. **Cleanup path:** if legacy `graph.json` / `profile.json` exist,
//!    delete them and emit one stderr line. Phase-3 migration shim.

use std::path::Path;

use gestalt::eigenvalue::EigenvalueProfile;
use gestalt::graph::ConceptGraph;

/// Result of loading or computing a concept graph.
pub struct CachedGraph {
    pub graph: ConceptGraph,
    pub profile: EigenvalueProfile,
    pub breakdown: gestalt::detect::GestaltBreakdown,
    /// True if the graph was loaded from git (the cache hit).
    pub from_cache: bool,
    /// The git OID of the spectral commit projected, if any.
    pub head_oid: Option<String>,
}

/// Compute a fast directory fingerprint: sorted file paths + sizes.
///
/// Retained for the gestalt-fallback path where git ref is absent.
/// Once a spectral commit exists, the git OID supersedes this.
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

/// Try to project the graph from `refs/spectral/HEAD`. Returns None if the
/// repo or ref is missing. Errors during traversal are logged (eprintln) and
/// also yield None so we fall back to the gestalt scan.
pub fn load_from_git(path: &Path) -> Option<CachedGraph> {
    let repo = git2::Repository::open(path).ok()?;

    let reference = repo
        .find_reference("refs/spectral/HEAD")
        .or_else(|_| repo.find_reference("refs/spectral/head"))
        .ok()?;

    let resolved = reference.resolve().ok().unwrap_or(reference);
    let commit = resolved.peel_to_commit().ok()?;
    let head_oid = commit.id().to_string();
    let root_tree = commit.tree().ok()?;

    let mut nodes: Vec<gestalt::graph::GraphNode> = Vec::new();
    let mut edges: Vec<gestalt::graph::GraphEdge> = Vec::new();
    let mut node_index_by_oid: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut edge_pairs: std::collections::HashSet<(usize, usize)> =
        std::collections::HashSet::new();
    let mut profile_blob_oid: Option<git2::Oid> = None;
    let mut profile_blob_content: Option<Vec<u8>> = None;

    // Phase 4 §3.3: per-node subtrees may live under `nodes/` instead of at
    // the root. Always read `profile` from the root; node entries from
    // whichever shape applies.
    if let Some(profile_entry) = root_tree.get_name("profile") {
        if profile_entry.kind() == Some(git2::ObjectType::Blob) {
            profile_blob_oid = Some(profile_entry.id());
            if let Ok(blob) = repo.find_blob(profile_entry.id()) {
                profile_blob_content = Some(blob.content().to_vec());
            }
        }
    }
    let nested_nodes_id = root_tree
        .get_name("nodes")
        .filter(|e| e.kind() == Some(git2::ObjectType::Tree))
        .map(|e| e.id());
    let tree: git2::Tree<'_> = match nested_nodes_id {
        Some(id) => repo.find_tree(id).ok()?,
        None => root_tree.clone(),
    };

    // First pass: collect node OIDs (entries that are sub-trees and don't
    // start with `.`). Top-level non-tree entries on the legacy flat shape
    // (`profile`, `schema`, `manifest`) are sibling metadata blobs and are
    // already handled above.
    for entry in tree.iter() {
        let name = match entry.name() {
            Some(n) => n.to_string(),
            None => continue,
        };

        match (entry.kind(), name.as_str()) {
            (Some(git2::ObjectType::Blob), "profile") if nested_nodes_id.is_none() => {
                profile_blob_oid = Some(entry.id());
                if let Ok(obj) = entry.to_object(&repo) {
                    if let Ok(blob) = obj.peel_to_blob() {
                        profile_blob_content = Some(blob.content().to_vec());
                    }
                }
            }
            (Some(git2::ObjectType::Tree), _) if !name.starts_with('.') => {
                let idx = nodes.len();
                node_index_by_oid.insert(name.clone(), idx);
                // Synthesize a Directory-shaped node so the existing views
                // (which only inspect .nodes.len()) keep working. The semantic
                // identity (typed by spectral-db) is preserved by name == oid.
                nodes.push(gestalt::graph::GraphNode::Directory {
                    path: std::path::PathBuf::from(&name),
                    name: name.clone(),
                    depth: 0,
                    file_count: 0,
                });
            }
            _ => {}
        }
    }

    // Second pass: walk each per-node subtree, harvest edges (entries whose
    // name is another node OID, not a `.dot` metadata file).
    for entry in tree.iter() {
        let name = match entry.name() {
            Some(n) => n.to_string(),
            None => continue,
        };
        if name.starts_with('.') || entry.kind() != Some(git2::ObjectType::Tree) {
            continue;
        }
        let from_idx = match node_index_by_oid.get(&name) {
            Some(&i) => i,
            None => continue,
        };

        let subtree = match entry.to_object(&repo).and_then(|o| o.peel_to_tree()) {
            Ok(t) => t,
            Err(_) => continue,
        };

        for sub_entry in subtree.iter() {
            let sub_name = match sub_entry.name() {
                Some(n) => n,
                None => continue,
            };
            if sub_name.starts_with('.') {
                continue; // .type, .content, .ts, .meta — not edges
            }
            let to_idx = match node_index_by_oid.get(sub_name) {
                Some(&i) => i,
                None => continue, // edge to a node not in this commit (skip)
            };

            // Read edge weight if present (best-effort)
            let weight = sub_entry
                .to_object(&repo)
                .ok()
                .and_then(|o| o.peel_to_blob().ok())
                .and_then(|b| {
                    let bytes = b.content();
                    // Try Edge JSON first, fall back to plain weight number.
                    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(bytes) {
                        v.get("weight").and_then(|w| w.as_f64())
                    } else {
                        std::str::from_utf8(bytes).ok().and_then(|s| s.parse().ok())
                    }
                })
                .unwrap_or(1.0);

            // Canonicalize undirected edge: store min/max once.
            let (lo, hi) = if from_idx <= to_idx {
                (from_idx, to_idx)
            } else {
                (to_idx, from_idx)
            };
            if edge_pairs.insert((lo, hi)) {
                edges.push(gestalt::graph::GraphEdge::SimilarContent {
                    a_idx: lo,
                    b_idx: hi,
                    weight,
                });
            }
        }
    }

    let graph = ConceptGraph { nodes, edges };
    let profile = profile_blob_content
        .as_deref()
        .and_then(decode_profile_blob)
        .unwrap_or_else(|| gestalt::eigenvalue::eigenvalue_profile(&graph));

    let breakdown = gestalt::detect::GestaltBreakdown::default();

    let _ = profile_blob_oid; // reserved for Phase 4 — profile_oid surfaces in views.

    cleanup_legacy_json(path);

    Some(CachedGraph {
        graph,
        profile,
        breakdown,
        from_cache: true,
        head_oid: Some(head_oid),
    })
}

/// Decode a Phase 3 profile blob: 16 little-endian f64s, optionally headed
/// by a `spectral-profile\0` magic. Returns None on malformed input.
fn decode_profile_blob(bytes: &[u8]) -> Option<EigenvalueProfile> {
    let payload = if let Some(rest) = bytes.strip_prefix(b"spectral-profile\0") {
        // Skip optional ASCII metadata block until we find a blank line.
        let after_blank = rest.windows(2).position(|w| w == b"\n\n");
        match after_blank {
            Some(i) => &rest[i + 2..],
            None => rest,
        }
    } else {
        bytes
    };
    if payload.len() < 16 * 8 {
        return None;
    }
    let mut values = [0.0f64; 16];
    for (i, chunk) in payload.chunks_exact(8).take(16).enumerate() {
        let arr: [u8; 8] = chunk.try_into().ok()?;
        values[i] = f64::from_le_bytes(arr);
    }
    Some(EigenvalueProfile { values })
}

/// Build the concept graph via the gestalt scan (used when no spectral commit exists).
pub fn build_and_cache(path: &Path) -> CachedGraph {
    let (graph, _files, breakdown) = gestalt::graph::build_concept_graph(path);
    let profile = gestalt::eigenvalue::eigenvalue_profile(&graph);

    cleanup_legacy_json(path);

    CachedGraph {
        graph,
        profile,
        breakdown,
        from_cache: false,
        head_oid: None,
    }
}

/// Project the graph from `refs/spectral/HEAD` if it exists; otherwise build
/// from gestalt scan. **No JSON cache is written.** The git OID is the cache
/// key; rebuilds happen automatically on commit advancement.
pub fn load_or_build(path: &Path) -> CachedGraph {
    if let Some(cached) = load_from_git(path) {
        return cached;
    }
    build_and_cache(path)
}

/// Delete legacy stopgap JSON files if present. Idempotent, silent on absence.
/// Logs one stderr line on first cleanup.
fn cleanup_legacy_json(path: &Path) {
    let contexts = path.join(".git/spectral/contexts");
    let graph_json = contexts.join("graph.json");
    let profile_json = contexts.join("profile.json");
    let mut cleaned = false;
    if graph_json.exists() {
        let _ = std::fs::remove_file(&graph_json);
        cleaned = true;
    }
    if profile_json.exists() {
        let _ = std::fs::remove_file(&profile_json);
        cleaned = true;
    }
    if cleaned {
        eprintln!(
            "spectral: removed legacy .git/spectral/contexts/{{graph,profile}}.json (Phase 3 migration)"
        );
    }
}

/// Phase-3 deprecated no-op. The git tree at `refs/spectral/HEAD` is the
/// source of truth; spectral-db's `settle()` writes it. Leaving this as a
/// no-op shim keeps existing call sites compiling during the transition.
#[deprecated(
    since = "0.2.0",
    note = "Graph state is git-native; spectral-db owns the write. This is a no-op."
)]
pub fn write_graph_cache(
    _path: &Path,
    _graph: &ConceptGraph,
    _profile: &EigenvalueProfile,
    _breakdown: &gestalt::detect::GestaltBreakdown,
) -> Result<(), String> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test_repo(path: &Path) -> git2::Repository {
        let repo = git2::Repository::init(path).unwrap();
        // Need an empty initial commit so refs work.
        let sig = git2::Signature::now("test", "test@local").unwrap();
        let tree_id = {
            let tb = repo.treebuilder(None).unwrap();
            tb.write().unwrap()
        };
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();
        }
        repo
    }

    /// Build a tiny per-node subtree with .type, .content, .ts, and edges to peers.
    fn write_node_subtree(
        repo: &git2::Repository,
        node_type: &str,
        content: &[u8],
        ts: &str,
        edges: &[(&str, &str)], // (target_oid, edge_blob_content)
    ) -> git2::Oid {
        let mut tb = repo.treebuilder(None).unwrap();
        let type_oid = repo.blob(node_type.as_bytes()).unwrap();
        tb.insert(".type", type_oid, 0o100644).unwrap();
        let content_oid = repo.blob(content).unwrap();
        tb.insert(".content", content_oid, 0o100644).unwrap();
        let ts_oid = repo.blob(ts.as_bytes()).unwrap();
        tb.insert(".ts", ts_oid, 0o100644).unwrap();
        for (target, blob_content) in edges {
            let edge_oid = repo.blob(blob_content.as_bytes()).unwrap();
            tb.insert(target, edge_oid, 0o100644).unwrap();
        }
        tb.write().unwrap()
    }

    fn write_graph_commit(
        repo: &git2::Repository,
        nodes: &[(&str, git2::Oid)], // (oid_name, subtree_oid)
        profile_bytes: Option<&[u8]>,
    ) -> git2::Oid {
        let mut tb = repo.treebuilder(None).unwrap();
        for (name, subtree) in nodes {
            tb.insert(*name, *subtree, 0o040000).unwrap();
        }
        if let Some(bytes) = profile_bytes {
            let blob = repo.blob(bytes).unwrap();
            tb.insert("profile", blob, 0o100644).unwrap();
        }
        let tree_oid = tb.write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("test", "test@local").unwrap();
        let commit_oid = repo
            .commit(None, &sig, &sig, "graph", &tree, &[])
            .unwrap();
        repo.reference(
            "refs/spectral/heads/main",
            commit_oid,
            true,
            "graph commit",
        )
        .unwrap();
        repo.reference_symbolic(
            "refs/spectral/HEAD",
            "refs/spectral/heads/main",
            true,
            "symref",
        )
        .unwrap();
        commit_oid
    }

    #[test]
    fn load_from_git_reads_nodes_and_edges() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(tmp.path());

        // Two nodes, one edge between them.
        let a_tree = write_node_subtree(
            &repo,
            "observation",
            b"alpha",
            "1000,1000",
            &[("oid_b", "0.7")],
        );
        let b_tree = write_node_subtree(
            &repo,
            "observation",
            b"beta",
            "2000,2000",
            &[("oid_a", "0.7")],
        );
        write_graph_commit(&repo, &[("oid_a", a_tree), ("oid_b", b_tree)], None);

        let cached = load_from_git(tmp.path()).expect("git load should succeed");
        assert_eq!(cached.graph.nodes.len(), 2);
        assert_eq!(cached.graph.edges.len(), 1, "bidirectional edge collapses to one");
        assert!(cached.from_cache);
        assert!(cached.head_oid.is_some());
    }

    #[test]
    fn load_from_git_skips_dotted_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(tmp.path());

        // Single isolated node — its .type/.content/.ts must NOT be counted as edges.
        let a_tree = write_node_subtree(&repo, "token", b"x", "10,10", &[]);
        write_graph_commit(&repo, &[("oid_a", a_tree)], None);

        let cached = load_from_git(tmp.path()).unwrap();
        assert_eq!(cached.graph.nodes.len(), 1);
        assert_eq!(cached.graph.edges.len(), 0);
    }

    #[test]
    fn load_or_build_falls_back_when_no_ref() {
        let tmp = tempfile::tempdir().unwrap();
        // No git repo at all — should still produce a valid CachedGraph via gestalt scan.
        std::fs::write(tmp.path().join("readme.md"), "# Hi").unwrap();
        let cached = load_or_build(tmp.path());
        assert!(!cached.from_cache);
        assert!(cached.head_oid.is_none());
    }

    #[test]
    fn load_or_build_prefers_git_when_present() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(tmp.path());
        let a_tree = write_node_subtree(&repo, "token", b"x", "0,0", &[]);
        write_graph_commit(&repo, &[("oid_a", a_tree)], None);

        let cached = load_or_build(tmp.path());
        assert!(cached.from_cache);
        assert_eq!(cached.graph.nodes.len(), 1);
    }

    #[test]
    fn load_or_build_does_not_write_json() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(tmp.path());
        let a_tree = write_node_subtree(&repo, "token", b"x", "0,0", &[]);
        write_graph_commit(&repo, &[("oid_a", a_tree)], None);

        let _ = load_or_build(tmp.path());
        let contexts = tmp.path().join(".git/spectral/contexts");
        assert!(
            !contexts.join("graph.json").exists(),
            "graph.json must not be created"
        );
        assert!(
            !contexts.join("profile.json").exists(),
            "profile.json must not be created"
        );
    }

    #[test]
    fn cleanup_removes_legacy_json() {
        let tmp = tempfile::tempdir().unwrap();
        let contexts = tmp.path().join(".git/spectral/contexts");
        std::fs::create_dir_all(&contexts).unwrap();
        std::fs::write(contexts.join("graph.json"), "stale").unwrap();
        std::fs::write(contexts.join("profile.json"), "stale").unwrap();
        cleanup_legacy_json(tmp.path());
        assert!(!contexts.join("graph.json").exists());
        assert!(!contexts.join("profile.json").exists());
    }

    #[test]
    fn legacy_head_ref_is_resolved() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(tmp.path());
        // Build like Phase 1: only `refs/spectral/head`, no HEAD/heads/main.
        let a_tree = write_node_subtree(&repo, "token", b"x", "0,0", &[]);
        let mut tb = repo.treebuilder(None).unwrap();
        tb.insert("oid_a", a_tree, 0o040000).unwrap();
        let tree_oid = tb.write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("test", "test@local").unwrap();
        repo.commit(
            Some("refs/spectral/head"),
            &sig,
            &sig,
            "phase1",
            &tree,
            &[],
        )
        .unwrap();

        let cached = load_from_git(tmp.path()).expect("legacy ref should be resolved");
        assert_eq!(cached.graph.nodes.len(), 1);
    }

    #[test]
    fn dir_hash_deterministic() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.md"), "# Hello").unwrap();
        std::fs::write(tmp.path().join("b.rs"), "fn main() {}").unwrap();
        let h1 = dir_hash(tmp.path());
        let h2 = dir_hash(tmp.path());
        assert_eq!(h1, h2);
    }

    #[test]
    fn write_graph_cache_is_no_op() {
        let tmp = tempfile::tempdir().unwrap();
        let (graph, _files, breakdown) = gestalt::graph::build_concept_graph(tmp.path());
        let profile = gestalt::eigenvalue::eigenvalue_profile(&graph);
        #[allow(deprecated)]
        let result = write_graph_cache(tmp.path(), &graph, &profile, &breakdown);
        assert!(result.is_ok());
        // Nothing should be written.
        let contexts = tmp.path().join(".git/spectral/contexts");
        assert!(!contexts.join("graph.json").exists());
    }

    /// H12 (Phase 4): the reader handles the `nodes/` wrapper introduced
    /// by spectral-db's Phase 4 settle().
    #[test]
    fn graph_cache_reads_nodes_wrapper() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = init_test_repo(tmp.path());

        // Build per-node subtrees under nodes/ (Phase 4 shape).
        let a_tree = write_node_subtree(
            &repo,
            "observation",
            b"alpha",
            "0,0",
            &[("oid_b", "0.7")],
        );
        let b_tree = write_node_subtree(
            &repo,
            "observation",
            b"beta",
            "0,0",
            &[("oid_a", "0.7")],
        );
        let mut nodes_tb = repo.treebuilder(None).unwrap();
        nodes_tb.insert("oid_a", a_tree, 0o040000).unwrap();
        nodes_tb.insert("oid_b", b_tree, 0o040000).unwrap();
        let nodes_tree = nodes_tb.write().unwrap();

        let mut profile_bytes = Vec::new();
        profile_bytes.extend_from_slice(b"spectral-profile\0");
        profile_bytes.extend_from_slice(b"fiedler: 0.0\nnodes:   2\nedges:   1\n\n");
        for _ in 0..16 {
            profile_bytes.extend_from_slice(&0.0f64.to_le_bytes());
        }
        profile_bytes.extend_from_slice(&[0u8; 32]);

        let mut root_tb = repo.treebuilder(None).unwrap();
        root_tb.insert("nodes", nodes_tree, 0o040000).unwrap();
        let pb = repo.blob(&profile_bytes).unwrap();
        root_tb.insert("profile", pb, 0o100644).unwrap();
        let tree_oid = root_tb.write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("test", "test@local").unwrap();
        let commit_oid = repo.commit(None, &sig, &sig, "phase4 nodes/", &tree, &[]).unwrap();
        repo.reference("refs/spectral/heads/main", commit_oid, true, "init").unwrap();
        repo.reference_symbolic(
            "refs/spectral/HEAD",
            "refs/spectral/heads/main",
            true,
            "symref",
        )
        .unwrap();

        let cached = load_from_git(tmp.path()).expect("nodes/ wrapper must load");
        assert_eq!(cached.graph.nodes.len(), 2, "two nodes");
        assert_eq!(cached.graph.edges.len(), 1, "one edge");
        assert!(cached.from_cache);
        assert!(cached.head_oid.is_some());
    }

    #[test]
    fn decode_profile_blob_round_trip() {
        let mut bytes = Vec::new();
        for i in 0..16 {
            bytes.extend_from_slice(&(i as f64 * 0.1).to_le_bytes());
        }
        let decoded = decode_profile_blob(&bytes).unwrap();
        for i in 0..16 {
            assert!((decoded.values[i] - i as f64 * 0.1).abs() < 1e-12);
        }
    }
}
