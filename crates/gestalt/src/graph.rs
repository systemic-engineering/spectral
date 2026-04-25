//! graph — concept graph from detected files.
//!
//! Directory-level nodes, structural edges, cross-reference edges.
//! The adjacency matrix that eigenvalue decomposition operates on.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use prism_core::oid::Oid;

use crate::detect::{
    walk_detected, extract_markdown_shape, DetectedFile, GestaltBreakdown, GrammarKind,
};

// ---------------------------------------------------------------------------
// Node and Edge types
// ---------------------------------------------------------------------------

/// A node in the concept graph.
#[derive(Clone, Debug)]
pub enum GraphNode {
    /// A directory in the repo.
    Directory {
        path: PathBuf,
        name: String,
        depth: usize,
        file_count: u32,
    },
    /// Root node representing the entire repo.
    Root {
        path: PathBuf,
        file_count: u32,
    },
}

impl GraphNode {
    /// Content-addressed identity of this node.
    pub fn oid(&self) -> Oid {
        match self {
            GraphNode::Directory { path, file_count, .. } => {
                Oid::hash(format!("dir:{}:{}", path.display(), file_count).as_bytes())
            }
            GraphNode::Root { path, file_count } => {
                Oid::hash(format!("root:{}:{}", path.display(), file_count).as_bytes())
            }
        }
    }

    pub fn name(&self) -> &str {
        match self {
            GraphNode::Directory { name, .. } => name,
            GraphNode::Root { .. } => "<root>",
        }
    }

    pub fn file_count(&self) -> u32 {
        match self {
            GraphNode::Directory { file_count, .. } => *file_count,
            GraphNode::Root { file_count, .. } => *file_count,
        }
    }
}

/// An edge in the concept graph.
#[derive(Clone, Debug)]
pub enum GraphEdge {
    /// Structural: parent directory contains child directory.
    Contains {
        parent_idx: usize,
        child_idx: usize,
        weight: f64,
    },
    /// Directories share similar file type distributions.
    SimilarContent {
        a_idx: usize,
        b_idx: usize,
        weight: f64,
    },
    /// Cross-reference: files in dir A link to files in dir B.
    CrossRef {
        source_idx: usize,
        target_idx: usize,
        weight: f64,
    },
}

impl GraphEdge {
    pub fn indices(&self) -> (usize, usize) {
        match self {
            GraphEdge::Contains { parent_idx, child_idx, .. } => (*parent_idx, *child_idx),
            GraphEdge::SimilarContent { a_idx, b_idx, .. } => (*a_idx, *b_idx),
            GraphEdge::CrossRef { source_idx, target_idx, .. } => (*source_idx, *target_idx),
        }
    }

    pub fn weight(&self) -> f64 {
        match self {
            GraphEdge::Contains { weight, .. } => *weight,
            GraphEdge::SimilarContent { weight, .. } => *weight,
            GraphEdge::CrossRef { weight, .. } => *weight,
        }
    }
}

// ---------------------------------------------------------------------------
// ConceptGraph
// ---------------------------------------------------------------------------

/// A concept graph: directory-level nodes with structural and semantic edges.
#[derive(Clone, Debug)]
pub struct ConceptGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl ConceptGraph {
    /// Empty graph.
    pub fn empty() -> Self {
        ConceptGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Content-addressed identity of the entire graph.
    pub fn oid(&self) -> Oid {
        let mut content = String::from("concept_graph:");
        let mut node_oids: Vec<String> = self.nodes.iter().map(|n| n.oid().to_string()).collect();
        node_oids.sort();
        content.push_str(&node_oids.join(":"));
        content.push(':');
        for edge in &self.edges {
            let (a, b) = edge.indices();
            content.push_str(&format!("{}->{}:{:.4},", a, b, edge.weight()));
        }
        Oid::hash(content.as_bytes())
    }

    /// Build the adjacency matrix (symmetric, weighted).
    /// Returns (matrix in row-major flat layout, dimension n).
    pub fn adjacency_matrix(&self) -> (Vec<f64>, usize) {
        let n = self.nodes.len();
        if n == 0 {
            return (Vec::new(), 0);
        }

        let mut matrix = vec![0.0_f64; n * n];

        for edge in &self.edges {
            let (i, j) = edge.indices();
            let w = edge.weight();
            if i < n && j < n {
                matrix[i * n + j] += w;
                matrix[j * n + i] += w;
            }
        }

        (matrix, n)
    }

    /// Build the Laplacian matrix (D - A) for eigenvalue decomposition.
    /// Returns (matrix in row-major flat layout, dimension n).
    pub fn laplacian_matrix(&self) -> (Vec<f64>, usize) {
        let n = self.nodes.len();
        if n == 0 {
            return (Vec::new(), 0);
        }

        let (adj, _) = self.adjacency_matrix();
        let mut laplacian = vec![0.0_f64; n * n];

        for i in 0..n {
            let mut degree = 0.0;
            for j in 0..n {
                let w = adj[i * n + j];
                if i != j {
                    laplacian[i * n + j] = -w;
                    degree += w;
                }
            }
            laplacian[i * n + i] = degree;
        }

        (laplacian, n)
    }
}

// ---------------------------------------------------------------------------
// Build a ConceptGraph from a directory
// ---------------------------------------------------------------------------

/// Build a directory-level concept graph from a path.
/// Each directory becomes a node. Edges come from:
/// 1. Structural nesting (Contains)
/// 2. Similar file type distributions (SimilarContent)
/// 3. Cross-references from wiki-links in markdown (CrossRef)
pub fn build_concept_graph(root: &Path) -> (ConceptGraph, Vec<DetectedFile>, GestaltBreakdown) {
    let (files, breakdown) = walk_detected(root);

    if files.is_empty() {
        return (ConceptGraph::empty(), files, breakdown);
    }

    // Collect directories and their file counts
    let mut dir_files: HashMap<PathBuf, Vec<&DetectedFile>> = HashMap::new();

    for file in &files {
        let parent = file.path.parent().unwrap_or(root).to_path_buf();
        dir_files.entry(parent).or_default().push(file);
    }

    // Create nodes: one per directory
    let mut nodes = Vec::new();
    let mut dir_to_idx: HashMap<PathBuf, usize> = HashMap::new();

    // Root node
    let root_count = dir_files.get(root).map(|f| f.len() as u32).unwrap_or(0);
    nodes.push(GraphNode::Root {
        path: root.to_path_buf(),
        file_count: root_count,
    });
    dir_to_idx.insert(root.to_path_buf(), 0);

    // All other directories sorted for determinism
    let mut dirs: Vec<PathBuf> = dir_files.keys().cloned().collect();
    dirs.sort();

    for dir in &dirs {
        if dir == root {
            continue;
        }
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.to_string_lossy().to_string());
        let depth = dir
            .strip_prefix(root)
            .map(|r| r.components().count())
            .unwrap_or(0);
        let file_count = dir_files.get(dir).map(|f| f.len() as u32).unwrap_or(0);

        let idx = nodes.len();
        nodes.push(GraphNode::Directory {
            path: dir.clone(),
            name,
            depth,
            file_count,
        });
        dir_to_idx.insert(dir.clone(), idx);
    }

    let mut edges = Vec::new();

    // 1. Contains edges: parent dir -> child dir
    for dir in &dirs {
        if dir == root {
            continue;
        }
        let child_idx = match dir_to_idx.get(dir) {
            Some(&idx) => idx,
            None => continue,
        };
        // Find parent directory
        let parent = dir.parent().unwrap_or(root);
        let parent_idx = if let Some(&idx) = dir_to_idx.get(parent) {
            idx
        } else {
            // Parent might not be in the map if it has no files directly
            // Walk up to find nearest ancestor
            let mut ancestor = parent.to_path_buf();
            loop {
                if let Some(&idx) = dir_to_idx.get(&ancestor) {
                    break idx;
                }
                match ancestor.parent() {
                    Some(p) => ancestor = p.to_path_buf(),
                    None => break 0, // root
                }
            }
        };

        edges.push(GraphEdge::Contains {
            parent_idx,
            child_idx,
            weight: 1.0,
        });
    }

    // 2. SimilarContent edges: directories with overlapping file type distributions
    let dir_type_distributions = compute_type_distributions(&dir_files);
    let dir_indices: Vec<(PathBuf, usize)> = dir_to_idx.iter().map(|(p, &i)| (p.clone(), i)).collect();

    for i in 0..dir_indices.len() {
        for j in (i + 1)..dir_indices.len() {
            let (ref path_a, idx_a) = dir_indices[i];
            let (ref path_b, idx_b) = dir_indices[j];
            if let (Some(dist_a), Some(dist_b)) = (
                dir_type_distributions.get(path_a),
                dir_type_distributions.get(path_b),
            ) {
                let sim = cosine_similarity(dist_a, dist_b);
                if sim > 0.3 {
                    edges.push(GraphEdge::SimilarContent {
                        a_idx: idx_a,
                        b_idx: idx_b,
                        weight: sim * 0.5, // lower weight than structural edges
                    });
                }
            }
        }
    }

    // 3. CrossRef edges: wiki-links in markdown files pointing across directories
    let cross_refs = extract_cross_references(root, &files, &dir_to_idx);
    edges.extend(cross_refs);

    (
        ConceptGraph { nodes, edges },
        files,
        breakdown,
    )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Type distribution for a directory: count of files per GrammarKind category.
fn compute_type_distributions(
    dir_files: &HashMap<PathBuf, Vec<&DetectedFile>>,
) -> HashMap<PathBuf, Vec<f64>> {
    // Categories: markdown, code, config, asset, gestalt, mirror, unknown
    const N_CATEGORIES: usize = 7;
    let mut distributions: HashMap<PathBuf, Vec<f64>> = HashMap::new();

    for (dir, files) in dir_files {
        let mut counts = vec![0.0_f64; N_CATEGORIES];
        for file in files {
            match &file.kind {
                GrammarKind::Markdown => counts[0] += 1.0,
                GrammarKind::Code(_) => counts[1] += 1.0,
                GrammarKind::Config(_) => counts[2] += 1.0,
                GrammarKind::Asset => counts[3] += 1.0,
                GrammarKind::GestaltNative => counts[4] += 1.0,
                GrammarKind::Mirror => counts[5] += 1.0,
                GrammarKind::Unknown => counts[6] += 1.0,
            }
        }
        distributions.insert(dir.clone(), counts);
    }

    distributions
}

/// Cosine similarity between two distributions.
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

/// Extract cross-reference edges from wiki-links in markdown files.
fn extract_cross_references(
    root: &Path,
    files: &[DetectedFile],
    dir_to_idx: &HashMap<PathBuf, usize>,
) -> Vec<GraphEdge> {
    let mut edges = Vec::new();

    // Build file name -> directory index map
    let mut file_name_to_dir: HashMap<String, usize> = HashMap::new();
    for file in files {
        let name = file
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let dir = file.path.parent().unwrap_or(root);
        if let Some(&idx) = dir_to_idx.get(dir) {
            file_name_to_dir.insert(name, idx);
        }
    }

    // Scan markdown files for wiki-links
    for file in files {
        if file.kind != GrammarKind::Markdown {
            continue;
        }
        let content = match std::fs::read_to_string(&file.path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let shape = extract_markdown_shape(&content);
        let source_dir = file.path.parent().unwrap_or(root);
        let source_idx = match dir_to_idx.get(source_dir) {
            Some(&idx) => idx,
            None => continue,
        };

        for target_name in &shape.wiki_link_targets {
            // Normalize target name (remove path separators, extensions)
            let normalized = Path::new(target_name)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| target_name.clone());

            if let Some(&target_idx) = file_name_to_dir.get(&normalized) {
                if target_idx != source_idx {
                    edges.push(GraphEdge::CrossRef {
                        source_idx,
                        target_idx,
                        weight: 0.3,
                    });
                }
            }
        }
    }

    edges
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn concept_graph_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let (graph, files, breakdown) = build_concept_graph(dir.path());
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
        assert_eq!(files.len(), 0);
        assert_eq!(breakdown.total(), 0);
    }

    #[test]
    fn concept_graph_single_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("readme.md"), "# Hello\n\nWorld\n").unwrap();

        let (graph, files, breakdown) = build_concept_graph(dir.path());
        assert_eq!(files.len(), 1);
        assert_eq!(breakdown.markdown, 1);
        // Should have at least the root node
        assert!(graph.nodes.len() >= 1);
    }

    #[test]
    fn concept_graph_directory_nesting() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("root.md"), "# Root").unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("child.md"), "# Child").unwrap();
        let deep = sub.join("deep");
        fs::create_dir(&deep).unwrap();
        fs::write(deep.join("leaf.md"), "# Leaf").unwrap();

        let (graph, _, _) = build_concept_graph(dir.path());

        // root + sub + deep = 3 directory nodes
        assert_eq!(graph.nodes.len(), 3);

        // Should have Contains edges
        let contains_count = graph
            .edges
            .iter()
            .filter(|e| matches!(e, GraphEdge::Contains { .. }))
            .count();
        assert!(contains_count >= 2, "expected at least 2 Contains edges, got {}", contains_count);
    }

    #[test]
    fn concept_graph_wiki_link_cross_ref() {
        let dir = tempfile::tempdir().unwrap();
        let docs = dir.path().join("docs");
        fs::create_dir(&docs).unwrap();
        let blog = dir.path().join("blog");
        fs::create_dir(&blog).unwrap();

        fs::write(docs.join("guide.md"), "See [[post]] for details.\n").unwrap();
        fs::write(blog.join("post.md"), "# My Post\n").unwrap();

        let (graph, _, _) = build_concept_graph(dir.path());

        // Should have a CrossRef edge from docs -> blog
        let cross_ref_count = graph
            .edges
            .iter()
            .filter(|e| matches!(e, GraphEdge::CrossRef { .. }))
            .count();
        assert!(cross_ref_count >= 1, "expected cross-ref edge for wiki-link");
    }

    #[test]
    fn concept_graph_similar_content_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let src1 = dir.path().join("src1");
        fs::create_dir(&src1).unwrap();
        let src2 = dir.path().join("src2");
        fs::create_dir(&src2).unwrap();

        // Both directories have code files -> should get SimilarContent edge
        fs::write(src1.join("a.rs"), "fn a() {}").unwrap();
        fs::write(src1.join("b.rs"), "fn b() {}").unwrap();
        fs::write(src2.join("c.rs"), "fn c() {}").unwrap();
        fs::write(src2.join("d.rs"), "fn d() {}").unwrap();

        let (graph, _, _) = build_concept_graph(dir.path());

        let similar_count = graph
            .edges
            .iter()
            .filter(|e| matches!(e, GraphEdge::SimilarContent { .. }))
            .count();
        assert!(similar_count >= 1, "expected SimilarContent edge between code dirs");
    }

    #[test]
    fn concept_graph_content_addressed() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "# Hello").unwrap();
        fs::write(dir.path().join("b.md"), "# World").unwrap();

        let (graph_a, _, _) = build_concept_graph(dir.path());
        let (graph_b, _, _) = build_concept_graph(dir.path());

        assert_eq!(graph_a.oid(), graph_b.oid());
    }

    #[test]
    fn concept_graph_different_dirs_different_oid() {
        let dir_a = tempfile::tempdir().unwrap();
        fs::write(dir_a.path().join("a.md"), "# Alpha").unwrap();

        let dir_b = tempfile::tempdir().unwrap();
        fs::write(dir_b.path().join("b.rs"), "fn main() {}").unwrap();

        let (graph_a, _, _) = build_concept_graph(dir_a.path());
        let (graph_b, _, _) = build_concept_graph(dir_b.path());

        assert_ne!(graph_a.oid(), graph_b.oid());
    }

    #[test]
    fn adjacency_matrix_empty() {
        let graph = ConceptGraph::empty();
        let (matrix, n) = graph.adjacency_matrix();
        assert_eq!(n, 0);
        assert!(matrix.is_empty());
    }

    #[test]
    fn adjacency_matrix_two_connected() {
        let graph = ConceptGraph {
            nodes: vec![
                GraphNode::Root {
                    path: PathBuf::from("/a"),
                    file_count: 1,
                },
                GraphNode::Directory {
                    path: PathBuf::from("/a/b"),
                    name: "b".into(),
                    depth: 1,
                    file_count: 1,
                },
            ],
            edges: vec![GraphEdge::Contains {
                parent_idx: 0,
                child_idx: 1,
                weight: 1.0,
            }],
        };

        let (matrix, n) = graph.adjacency_matrix();
        assert_eq!(n, 2);
        assert_eq!(matrix[0 * 2 + 1], 1.0); // (0,1) = 1.0
        assert_eq!(matrix[1 * 2 + 0], 1.0); // (1,0) = 1.0 (symmetric)
        assert_eq!(matrix[0 * 2 + 0], 0.0); // diagonal = 0
    }

    #[test]
    fn laplacian_matrix_two_connected() {
        let graph = ConceptGraph {
            nodes: vec![
                GraphNode::Root {
                    path: PathBuf::from("/a"),
                    file_count: 1,
                },
                GraphNode::Directory {
                    path: PathBuf::from("/a/b"),
                    name: "b".into(),
                    depth: 1,
                    file_count: 1,
                },
            ],
            edges: vec![GraphEdge::Contains {
                parent_idx: 0,
                child_idx: 1,
                weight: 1.0,
            }],
        };

        let (laplacian, n) = graph.laplacian_matrix();
        assert_eq!(n, 2);
        // L = D - A = [[1, -1], [-1, 1]]
        assert_eq!(laplacian[0], 1.0);
        assert_eq!(laplacian[1], -1.0);
        assert_eq!(laplacian[2], -1.0);
        assert_eq!(laplacian[3], 1.0);
    }

    #[test]
    fn cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-10);
    }
}
