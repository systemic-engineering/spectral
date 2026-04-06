//! Diff — compare two spectral states by eigenvalue.
//!
//! `compute` returns per-node and total growth delta between two states.
//! `format` renders the diff for display with directional arrows.

use std::collections::{HashMap, HashSet};

/// Growth state of a single node (0.0 = no growth, 1.0 = fully grown).
pub struct NodeState {
    pub name: String,
    pub depth: f64,
}

/// Overall state of a spectral session.
pub struct State {
    pub growth: f64,
    pub nodes: Vec<NodeState>,
}

/// Per-node delta between two states.
pub struct NodeDiff {
    pub name: String,
    pub delta: f64,
    pub from: f64,
    pub to: f64,
}

/// Full diff between two states.
pub struct Diff {
    pub total_delta: f64,
    pub per_node: Vec<NodeDiff>,
}

/// Compare two states, returning per-node and total delta.
///
/// Nodes missing from `a` are treated as depth 0.0.
/// Nodes missing from `b` are treated as depth 0.0.
pub fn compute(a: &State, b: &State) -> Diff {
    let a_map: HashMap<&str, f64> = a.nodes.iter().map(|n| (n.name.as_str(), n.depth)).collect();
    let b_map: HashMap<&str, f64> = b.nodes.iter().map(|n| (n.name.as_str(), n.depth)).collect();

    // Collect all unique node names from both states
    let mut all_names: HashSet<&str> = HashSet::new();
    for n in &a.nodes { all_names.insert(n.name.as_str()); }
    for n in &b.nodes { all_names.insert(n.name.as_str()); }

    let mut per_node: Vec<NodeDiff> = all_names
        .into_iter()
        .map(|name| {
            let from = *a_map.get(name).unwrap_or(&0.0);
            let to = *b_map.get(name).unwrap_or(&0.0);
            NodeDiff {
                name: name.to_string(),
                delta: to - from,
                from,
                to,
            }
        })
        .collect();

    // Stable ordering: sort by name
    per_node.sort_by(|x, y| x.name.cmp(&y.name));

    Diff {
        total_delta: b.growth - a.growth,
        per_node,
    }
}

/// Format a diff for display with directional arrows.
///
/// - `↑` for positive delta
/// - `↓` for negative delta
/// - `=` for zero delta
pub fn format(diff: &Diff) -> String {
    let arrow = |delta: f64| {
        if delta > 0.0 { '↑' } else if delta < 0.0 { '↓' } else { '=' }
    };

    let total_arrow = arrow(diff.total_delta);
    let mut lines = vec![
        format!(
            "total {} {:.1}% ({:+.1}%)",
            total_arrow,
            (diff.total_delta * 100.0).abs(),
            diff.total_delta * 100.0,
        )
    ];

    for n in &diff.per_node {
        lines.push(format!(
            "  {} {} {:.0}% → {:.0}% ({:+.1}%)",
            n.name,
            arrow(n.delta),
            n.from * 100.0,
            n.to * 100.0,
            n.delta * 100.0,
        ));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, depth: f64) -> NodeState {
        NodeState { name: name.to_string(), depth }
    }

    fn state(growth: f64, nodes: Vec<NodeState>) -> State {
        State { growth, nodes }
    }

    #[test]
    fn diff_identical_states_is_zero() {
        let a = state(0.5, vec![node("alpha", 0.3), node("beta", 0.7)]);
        let b = state(0.5, vec![node("alpha", 0.3), node("beta", 0.7)]);
        let d = compute(&a, &b);
        assert_eq!(d.total_delta, 0.0);
        assert!(d.per_node.iter().all(|n| n.delta == 0.0));
    }

    #[test]
    fn diff_growth_difference() {
        let a = state(0.33, vec![node("alpha", 0.2), node("beta", 0.5)]);
        let b = state(0.59, vec![node("alpha", 0.5), node("beta", 0.7)]);
        let d = compute(&a, &b);
        // total delta = 0.59 - 0.33 = 0.26
        let total = (d.total_delta * 100.0).round() / 100.0;
        assert_eq!(total, 0.26);
        // per-node: alpha delta = 0.3, beta delta = 0.2
        let alpha = d.per_node.iter().find(|n| n.name == "alpha").unwrap();
        let beta = d.per_node.iter().find(|n| n.name == "beta").unwrap();
        let alpha_delta = (alpha.delta * 10.0).round() / 10.0;
        let beta_delta = (beta.delta * 10.0).round() / 10.0;
        assert_eq!(alpha_delta, 0.3);
        assert_eq!(beta_delta, 0.2);
    }

    #[test]
    fn diff_new_node_in_b() {
        let a = state(0.4, vec![node("alpha", 0.4)]);
        let b = state(0.7, vec![node("alpha", 0.4), node("gamma", 0.6)]);
        let d = compute(&a, &b);
        // gamma is new in b, so from=0.0, delta=0.6
        let gamma = d.per_node.iter().find(|n| n.name == "gamma").unwrap();
        assert_eq!(gamma.from, 0.0);
        let gamma_delta = (gamma.delta * 10.0).round() / 10.0;
        assert_eq!(gamma_delta, 0.6);
    }

    #[test]
    fn diff_format_shows_arrows() {
        let a = state(0.3, vec![node("alpha", 0.2), node("beta", 0.5), node("gamma", 0.4)]);
        let b = state(0.6, vec![node("alpha", 0.7), node("beta", 0.3), node("gamma", 0.4)]);
        let d = compute(&a, &b);
        let output = format(&d);
        // alpha grew -> ↑
        assert!(output.contains('↑'), "expected ↑ in output: {}", output);
        // beta shrank -> ↓
        assert!(output.contains('↓'), "expected ↓ in output: {}", output);
        // gamma unchanged -> =
        assert!(output.contains('='), "expected = in output: {}", output);
    }
}
