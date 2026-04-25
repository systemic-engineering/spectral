//! eigenvalue — eigenvalue decomposition for concept graphs.
//!
//! Pure-Rust implementation for small matrices (< 500 nodes).
//! Uses the Jacobi eigenvalue algorithm for real symmetric matrices.
//! The Laplacian of the concept graph is always real symmetric.
//!
//! Produces a 16-value EigenvalueProfile: the top-16 eigenvalues of the
//! graph Laplacian, normalized to [0, 1]. The Fiedler value (second-smallest
//! eigenvalue) measures algebraic connectivity.

use prism_core::oid::Oid;

use crate::graph::ConceptGraph;

// ---------------------------------------------------------------------------
// EigenvalueProfile — the spectral fingerprint
// ---------------------------------------------------------------------------

/// A 16-value eigenvalue profile. The spectral fingerprint of a concept graph.
///
/// Values are normalized to [0.0, 1.0]. The profile captures the shape
/// of the graph's connectivity structure.
#[derive(Clone, Debug, PartialEq)]
pub struct EigenvalueProfile {
    pub values: [f64; 16],
}

impl EigenvalueProfile {
    /// All zeros — the dark profile.
    pub fn dark() -> Self {
        EigenvalueProfile {
            values: [0.0; 16],
        }
    }

    /// Check if this is a trivial (all-zero) profile.
    pub fn is_dark(&self) -> bool {
        self.values.iter().all(|&v| v == 0.0)
    }

    /// The Fiedler value: second-smallest eigenvalue of the Laplacian.
    /// Measures algebraic connectivity. Zero = disconnected graph.
    pub fn fiedler_value(&self) -> f64 {
        // The eigenvalues are stored smallest-first, so index 1 is the Fiedler value.
        // But after normalization, we need to use the un-normalized source.
        // Since we store normalized values, the Fiedler value is values[1].
        if self.values.len() < 2 {
            return 0.0;
        }
        self.values[1]
    }

    /// Content-addressed identity.
    pub fn oid(&self) -> Oid {
        let values_str: String = self
            .values
            .iter()
            .map(|v| format!("{:.8}", v))
            .collect::<Vec<_>>()
            .join(",");
        Oid::hash(format!("eigenvalue_profile:{}", values_str).as_bytes())
    }

    /// Serialize to bytes for snapshot inclusion.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(128);
        for &v in &self.values {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        buf
    }
}

// ---------------------------------------------------------------------------
// Eigenvalue decomposition
// ---------------------------------------------------------------------------

/// Compute the eigenvalue profile of a concept graph.
///
/// 1. Build the graph Laplacian (L = D - A).
/// 2. Compute eigenvalues using the Jacobi algorithm.
/// 3. Sort eigenvalues ascending.
/// 4. Take the first 16 (or pad with zeros).
/// 5. Normalize to [0, 1].
pub fn eigenvalue_profile(graph: &ConceptGraph) -> EigenvalueProfile {
    let n = graph.nodes.len();
    if n == 0 {
        return EigenvalueProfile::dark();
    }
    if n == 1 {
        // Degenerate: single node, no edges, all eigenvalues are 0
        return EigenvalueProfile::dark();
    }

    let (laplacian, dim) = graph.laplacian_matrix();
    let eigenvalues = jacobi_eigenvalues(&laplacian, dim);

    build_profile(&eigenvalues)
}

/// Build a 16-value normalized profile from raw eigenvalues.
fn build_profile(eigenvalues: &[f64]) -> EigenvalueProfile {
    let mut sorted = eigenvalues.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut values = [0.0_f64; 16];
    let count = sorted.len().min(16);
    for i in 0..count {
        values[i] = sorted[i];
    }

    // Normalize to [0, 1]
    let max = values.iter().cloned().fold(0.0_f64, f64::max);
    if max > 1e-12 {
        for v in &mut values {
            *v /= max;
        }
    }

    EigenvalueProfile { values }
}

// ---------------------------------------------------------------------------
// Jacobi eigenvalue algorithm for real symmetric matrices
// ---------------------------------------------------------------------------

/// Compute eigenvalues of a real symmetric matrix using the Jacobi method.
/// Input: row-major flat matrix, dimension n.
/// Returns eigenvalues in no particular order (caller should sort).
///
/// The Jacobi method is O(n^3) per sweep, converges in O(n^2) sweeps for
/// well-conditioned matrices. For n < 500, this is sub-second.
fn jacobi_eigenvalues(matrix: &[f64], n: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![matrix[0]];
    }

    // Copy matrix (we modify it in place)
    let mut a = matrix.to_vec();
    let max_iter = 100 * n * n;
    let eps = 1e-12;

    for _ in 0..max_iter {
        // Find the largest off-diagonal element
        let mut max_val = 0.0_f64;
        let mut p = 0;
        let mut q = 1;
        for i in 0..n {
            for j in (i + 1)..n {
                let val = a[i * n + j].abs();
                if val > max_val {
                    max_val = val;
                    p = i;
                    q = j;
                }
            }
        }

        // Convergence check
        if max_val < eps {
            break;
        }

        // Compute rotation angle
        let app = a[p * n + p];
        let aqq = a[q * n + q];
        let apq = a[p * n + q];

        let theta = if (app - aqq).abs() < eps {
            std::f64::consts::FRAC_PI_4
        } else {
            0.5 * ((2.0 * apq) / (app - aqq)).atan()
        };

        let cos_t = theta.cos();
        let sin_t = theta.sin();

        // Apply Givens rotation
        let mut new_a = a.clone();

        // Update rows and columns p and q
        for i in 0..n {
            if i != p && i != q {
                let aip = a[i * n + p];
                let aiq = a[i * n + q];
                new_a[i * n + p] = cos_t * aip + sin_t * aiq;
                new_a[p * n + i] = new_a[i * n + p];
                new_a[i * n + q] = -sin_t * aip + cos_t * aiq;
                new_a[q * n + i] = new_a[i * n + q];
            }
        }

        // Update diagonal and off-diagonal of p, q
        new_a[p * n + p] = cos_t * cos_t * app + 2.0 * cos_t * sin_t * apq + sin_t * sin_t * aqq;
        new_a[q * n + q] = sin_t * sin_t * app - 2.0 * cos_t * sin_t * apq + cos_t * cos_t * aqq;
        new_a[p * n + q] = 0.0;
        new_a[q * n + p] = 0.0;

        a = new_a;
    }

    // Eigenvalues are the diagonal elements
    (0..n).map(|i| a[i * n + i]).collect()
}

/// Compute eigenvalues AND eigenvectors of a real symmetric matrix using the Jacobi method.
/// Input: row-major flat matrix, dimension n.
/// Returns (eigenvalues, eigenvectors) where eigenvectors[i] is the eigenvector
/// for eigenvalues[i]. Both are sorted by eigenvalue ascending.
///
/// The Jacobi algorithm accumulates rotation matrices. The product of all
/// Givens rotations IS the eigenvector matrix. Column i of the product
/// corresponds to eigenvalue i.
pub fn jacobi_eigen_decomposition(matrix: &[f64], n: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    if n == 0 {
        return (Vec::new(), Vec::new());
    }
    if n == 1 {
        return (vec![matrix[0]], vec![vec![1.0]]);
    }

    // Copy matrix (we modify it in place)
    let mut a = matrix.to_vec();
    // Eigenvector matrix: starts as identity
    let mut v = vec![0.0_f64; n * n];
    for i in 0..n {
        v[i * n + i] = 1.0;
    }

    let max_iter = 100 * n * n;
    let eps = 1e-12;

    for _ in 0..max_iter {
        // Find the largest off-diagonal element
        let mut max_val = 0.0_f64;
        let mut p = 0;
        let mut q = 1;
        for i in 0..n {
            for j in (i + 1)..n {
                let val = a[i * n + j].abs();
                if val > max_val {
                    max_val = val;
                    p = i;
                    q = j;
                }
            }
        }

        // Convergence check
        if max_val < eps {
            break;
        }

        // Compute rotation angle
        let app = a[p * n + p];
        let aqq = a[q * n + q];
        let apq = a[p * n + q];

        let theta = if (app - aqq).abs() < eps {
            std::f64::consts::FRAC_PI_4
        } else {
            0.5 * ((2.0 * apq) / (app - aqq)).atan()
        };

        let cos_t = theta.cos();
        let sin_t = theta.sin();

        // Apply Givens rotation to A
        let mut new_a = a.clone();

        for i in 0..n {
            if i != p && i != q {
                let aip = a[i * n + p];
                let aiq = a[i * n + q];
                new_a[i * n + p] = cos_t * aip + sin_t * aiq;
                new_a[p * n + i] = new_a[i * n + p];
                new_a[i * n + q] = -sin_t * aip + cos_t * aiq;
                new_a[q * n + i] = new_a[i * n + q];
            }
        }

        new_a[p * n + p] = cos_t * cos_t * app + 2.0 * cos_t * sin_t * apq + sin_t * sin_t * aqq;
        new_a[q * n + q] = sin_t * sin_t * app - 2.0 * cos_t * sin_t * apq + cos_t * cos_t * aqq;
        new_a[p * n + q] = 0.0;
        new_a[q * n + p] = 0.0;

        a = new_a;

        // Accumulate rotation into eigenvector matrix: V = V * G
        // G only mixes columns p and q
        for i in 0..n {
            let vip = v[i * n + p];
            let viq = v[i * n + q];
            v[i * n + p] = cos_t * vip + sin_t * viq;
            v[i * n + q] = -sin_t * vip + cos_t * viq;
        }
    }

    // Collect eigenvalues and eigenvectors
    let eigenvalues: Vec<f64> = (0..n).map(|i| a[i * n + i]).collect();
    let eigenvectors: Vec<Vec<f64>> = (0..n)
        .map(|j| (0..n).map(|i| v[i * n + j]).collect())
        .collect();

    // Sort by eigenvalue ascending
    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by(|&a, &b| eigenvalues[a].partial_cmp(&eigenvalues[b]).unwrap_or(std::cmp::Ordering::Equal));

    let sorted_vals: Vec<f64> = indices.iter().map(|&i| eigenvalues[i]).collect();
    let sorted_vecs: Vec<Vec<f64>> = indices.iter().map(|&i| eigenvectors[i].clone()).collect();

    (sorted_vals, sorted_vecs)
}

/// Compute 2D spectral embedding of a concept graph.
///
/// Uses the Fiedler vector (eigenvector 2) as x-axis and the third eigenvector
/// as y-axis. Returns one [f32; 2] position per node, normalized to [-1, 1].
///
/// For graphs with < 3 nodes, returns zero positions.
pub fn spectral_embedding_2d(graph: &ConceptGraph) -> Vec<[f32; 2]> {
    let n = graph.nodes.len();
    if n < 3 {
        return vec![[0.0, 0.0]; n];
    }

    let (laplacian, dim) = graph.laplacian_matrix();
    let (_eigenvalues, eigenvectors) = jacobi_eigen_decomposition(&laplacian, dim);

    // Eigenvector index 1 = Fiedler vector (x), index 2 = third eigenvector (y)
    let fiedler = &eigenvectors[1];
    let third = &eigenvectors[2];

    // Find max absolute values for normalization
    let max_x = fiedler.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
    let max_y = third.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);

    let norm_x = if max_x > 1e-12 { max_x } else { 1.0 };
    let norm_y = if max_y > 1e-12 { max_y } else { 1.0 };

    (0..n)
        .map(|i| {
            [(fiedler[i] / norm_x) as f32, (third[i] / norm_y) as f32]
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphEdge, GraphNode};
    use std::path::PathBuf;

    #[test]
    fn eigenvalue_empty_graph() {
        let graph = ConceptGraph::empty();
        let profile = eigenvalue_profile(&graph);
        assert!(profile.is_dark());
        assert_eq!(profile.values, [0.0; 16]);
    }

    #[test]
    fn eigenvalue_single_node() {
        let graph = ConceptGraph {
            nodes: vec![GraphNode::Root {
                path: PathBuf::from("/test"),
                file_count: 1,
            }],
            edges: vec![],
        };
        let profile = eigenvalue_profile(&graph);
        assert!(profile.is_dark());
    }

    #[test]
    fn eigenvalue_two_disconnected() {
        let graph = ConceptGraph {
            nodes: vec![
                GraphNode::Root {
                    path: PathBuf::from("/a"),
                    file_count: 1,
                },
                GraphNode::Directory {
                    path: PathBuf::from("/b"),
                    name: "b".into(),
                    depth: 1,
                    file_count: 1,
                },
            ],
            edges: vec![], // no edges = disconnected
        };
        let profile = eigenvalue_profile(&graph);
        // Disconnected graph: all Laplacian eigenvalues are 0
        assert!(profile.is_dark(), "disconnected graph should have dark profile");
    }

    #[test]
    fn eigenvalue_two_connected() {
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
        let profile = eigenvalue_profile(&graph);
        // Connected 2-node graph: eigenvalues of L = [[1,-1],[-1,1]] are 0 and 2
        // After normalization: [0.0, 1.0, 0, 0, ...]
        assert!(!profile.is_dark());
        assert!((profile.values[0] - 0.0).abs() < 1e-6, "smallest eigenvalue should be ~0");
        assert!((profile.values[1] - 1.0).abs() < 1e-6, "second eigenvalue should be ~1 (normalized)");
    }

    #[test]
    fn eigenvalue_three_chain() {
        // Chain: A -- B -- C
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
                GraphNode::Directory {
                    path: PathBuf::from("/a/b/c"),
                    name: "c".into(),
                    depth: 2,
                    file_count: 1,
                },
            ],
            edges: vec![
                GraphEdge::Contains {
                    parent_idx: 0,
                    child_idx: 1,
                    weight: 1.0,
                },
                GraphEdge::Contains {
                    parent_idx: 1,
                    child_idx: 2,
                    weight: 1.0,
                },
            ],
        };
        let profile = eigenvalue_profile(&graph);
        assert!(!profile.is_dark());
        // 3-node path: eigenvalues of Laplacian are 0, 1, 3
        assert!((profile.values[0]).abs() < 1e-6, "smallest should be ~0, got {}", profile.values[0]);
        assert!(profile.values[1] > 0.0, "Fiedler value should be > 0");
    }

    #[test]
    fn eigenvalue_deterministic() {
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
        let profile_a = eigenvalue_profile(&graph);
        let profile_b = eigenvalue_profile(&graph);
        assert_eq!(profile_a.oid(), profile_b.oid());
    }

    #[test]
    fn eigenvalue_profile_content_addressed() {
        let p1 = EigenvalueProfile { values: [0.0, 0.5, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] };
        let p2 = EigenvalueProfile { values: [0.0, 0.5, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] };
        assert_eq!(p1.oid(), p2.oid());
    }

    #[test]
    fn eigenvalue_profile_different_values_different_oid() {
        let p1 = EigenvalueProfile { values: [0.0, 0.5, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] };
        let p2 = EigenvalueProfile { values: [0.0, 0.3, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] };
        assert_ne!(p1.oid(), p2.oid());
    }

    #[test]
    fn eigenvalue_profile_to_bytes() {
        let p = EigenvalueProfile { values: [1.0; 16] };
        let bytes = p.to_bytes();
        assert_eq!(bytes.len(), 128); // 16 * 8 bytes per f64
    }

    #[test]
    fn eigenvalue_fiedler_value() {
        let p = EigenvalueProfile {
            values: [0.0, 0.5, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        assert!((p.fiedler_value() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn jacobi_identity_matrix() {
        // Identity matrix: eigenvalues are all 1
        let matrix = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let evals = jacobi_eigenvalues(&matrix, 3);
        assert_eq!(evals.len(), 3);
        for &v in &evals {
            assert!((v - 1.0).abs() < 1e-10, "expected 1.0, got {}", v);
        }
    }

    #[test]
    fn jacobi_diagonal_matrix() {
        // Diagonal matrix with known eigenvalues
        let matrix = vec![3.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0];
        let mut evals = jacobi_eigenvalues(&matrix, 3);
        evals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((evals[0] - 1.0).abs() < 1e-10);
        assert!((evals[1] - 2.0).abs() < 1e-10);
        assert!((evals[2] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn jacobi_symmetric_2x2() {
        // [[2, 1], [1, 2]] -> eigenvalues 1, 3
        let matrix = vec![2.0, 1.0, 1.0, 2.0];
        let mut evals = jacobi_eigenvalues(&matrix, 2);
        evals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((evals[0] - 1.0).abs() < 1e-10, "expected 1.0, got {}", evals[0]);
        assert!((evals[1] - 3.0).abs() < 1e-10, "expected 3.0, got {}", evals[1]);
    }

    #[test]
    fn build_profile_from_graph_on_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# Title A\n\nContent\n").unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("b.md"), "# Title B\n\nContent\n").unwrap();

        let (graph, _, _) = crate::graph::build_concept_graph(dir.path());
        let profile = eigenvalue_profile(&graph);

        // Should produce a non-dark profile since we have a connected graph
        assert!(!profile.is_dark(), "profile should not be dark for connected graph");
    }

    // --- jacobi_eigen_decomposition tests ---

    #[test]
    fn decomposition_empty_matrix() {
        let (vals, vecs) = jacobi_eigen_decomposition(&[], 0);
        assert!(vals.is_empty());
        assert!(vecs.is_empty());
    }

    #[test]
    fn decomposition_1x1_matrix() {
        let (vals, vecs) = jacobi_eigen_decomposition(&[5.0], 1);
        assert_eq!(vals.len(), 1);
        assert!((vals[0] - 5.0).abs() < 1e-10);
        assert_eq!(vecs.len(), 1);
        assert!((vecs[0][0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn decomposition_identity_3x3() {
        let matrix = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let (vals, vecs) = jacobi_eigen_decomposition(&matrix, 3);
        assert_eq!(vals.len(), 3);
        assert_eq!(vecs.len(), 3);
        for &v in &vals {
            assert!((v - 1.0).abs() < 1e-10);
        }
        // Eigenvectors should be orthonormal
        for i in 0..3 {
            let norm: f64 = vecs[i].iter().map(|x| x * x).sum::<f64>().sqrt();
            assert!((norm - 1.0).abs() < 1e-8, "eigenvector {} should be unit length, got {}", i, norm);
        }
    }

    #[test]
    fn decomposition_symmetric_2x2_eigenvalues_correct() {
        // [[2, 1], [1, 2]] -> eigenvalues 1, 3
        let matrix = vec![2.0, 1.0, 1.0, 2.0];
        let (vals, vecs) = jacobi_eigen_decomposition(&matrix, 2);
        assert!((vals[0] - 1.0).abs() < 1e-10, "expected 1.0, got {}", vals[0]);
        assert!((vals[1] - 3.0).abs() < 1e-10, "expected 3.0, got {}", vals[1]);
        // Each eigenvector should have unit length
        for i in 0..2 {
            let norm: f64 = vecs[i].iter().map(|x| x * x).sum::<f64>().sqrt();
            assert!((norm - 1.0).abs() < 1e-8, "eigenvector {} norm = {}", i, norm);
        }
    }

    #[test]
    fn decomposition_eigenvectors_orthogonal() {
        // [[2, 1], [1, 2]] eigenvectors should be orthogonal
        let matrix = vec![2.0, 1.0, 1.0, 2.0];
        let (_vals, vecs) = jacobi_eigen_decomposition(&matrix, 2);
        let dot: f64 = vecs[0].iter().zip(vecs[1].iter()).map(|(a, b)| a * b).sum();
        assert!(dot.abs() < 1e-8, "eigenvectors should be orthogonal, dot = {}", dot);
    }

    #[test]
    fn decomposition_sorted_ascending() {
        let matrix = vec![3.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0];
        let (vals, _) = jacobi_eigen_decomposition(&matrix, 3);
        assert!((vals[0] - 1.0).abs() < 1e-10);
        assert!((vals[1] - 2.0).abs() < 1e-10);
        assert!((vals[2] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn decomposition_av_equals_lambda_v() {
        // Verify A*v = lambda*v for each eigenpair
        let matrix = vec![2.0, 1.0, 1.0, 2.0];
        let n = 2;
        let (vals, vecs) = jacobi_eigen_decomposition(&matrix, n);
        for k in 0..n {
            for i in 0..n {
                let av_i: f64 = (0..n).map(|j| matrix[i * n + j] * vecs[k][j]).sum();
                let lv_i = vals[k] * vecs[k][i];
                assert!(
                    (av_i - lv_i).abs() < 1e-8,
                    "A*v[{}][{}] = {}, lambda*v = {}", k, i, av_i, lv_i
                );
            }
        }
    }

    // --- spectral_embedding_2d tests ---

    #[test]
    fn embedding_small_graph_returns_zeros() {
        // 2-node graph: < 3 nodes, returns zero positions
        let graph = ConceptGraph {
            nodes: vec![
                GraphNode::Root { path: PathBuf::from("/a"), file_count: 1 },
                GraphNode::Directory { path: PathBuf::from("/a/b"), name: "b".into(), depth: 1, file_count: 1 },
            ],
            edges: vec![GraphEdge::Contains { parent_idx: 0, child_idx: 1, weight: 1.0 }],
        };
        let positions = spectral_embedding_2d(&graph);
        assert_eq!(positions.len(), 2);
        for p in &positions {
            assert_eq!(p[0], 0.0);
            assert_eq!(p[1], 0.0);
        }
    }

    #[test]
    fn embedding_three_chain_produces_positions() {
        let graph = ConceptGraph {
            nodes: vec![
                GraphNode::Root { path: PathBuf::from("/a"), file_count: 1 },
                GraphNode::Directory { path: PathBuf::from("/a/b"), name: "b".into(), depth: 1, file_count: 1 },
                GraphNode::Directory { path: PathBuf::from("/a/b/c"), name: "c".into(), depth: 2, file_count: 1 },
            ],
            edges: vec![
                GraphEdge::Contains { parent_idx: 0, child_idx: 1, weight: 1.0 },
                GraphEdge::Contains { parent_idx: 1, child_idx: 2, weight: 1.0 },
            ],
        };
        let positions = spectral_embedding_2d(&graph);
        assert_eq!(positions.len(), 3);
        // Positions should be in [-1, 1] range
        for p in &positions {
            assert!(p[0].abs() <= 1.0 + 1e-6, "x out of range: {}", p[0]);
            assert!(p[1].abs() <= 1.0 + 1e-6, "y out of range: {}", p[1]);
        }
        // Not all the same (connected graph with structure)
        let all_same = positions.windows(2).all(|w| (w[0][0] - w[1][0]).abs() < 1e-6);
        assert!(!all_same, "positions should differ for chain graph");
    }

    #[test]
    fn embedding_deterministic() {
        let graph = ConceptGraph {
            nodes: vec![
                GraphNode::Root { path: PathBuf::from("/a"), file_count: 1 },
                GraphNode::Directory { path: PathBuf::from("/a/b"), name: "b".into(), depth: 1, file_count: 1 },
                GraphNode::Directory { path: PathBuf::from("/a/c"), name: "c".into(), depth: 1, file_count: 1 },
            ],
            edges: vec![
                GraphEdge::Contains { parent_idx: 0, child_idx: 1, weight: 1.0 },
                GraphEdge::Contains { parent_idx: 0, child_idx: 2, weight: 1.0 },
            ],
        };
        let pos_a = spectral_embedding_2d(&graph);
        let pos_b = spectral_embedding_2d(&graph);
        for (a, b) in pos_a.iter().zip(pos_b.iter()) {
            assert!((a[0] - b[0]).abs() < 1e-10, "x not deterministic");
            assert!((a[1] - b[1]).abs() < 1e-10, "y not deterministic");
        }
    }
}
