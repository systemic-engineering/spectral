//! spectral — eigenvalue visualization block types.
//!
//! These are the render primitives for spectral data visualization.
//! Each block type renders as a DOM subtree via Panel trait.

use prism_core::oid::{Addressable, Oid};

// ---------------------------------------------------------------------------
// EigenvalueProfile — 16-value sparkline
// ---------------------------------------------------------------------------

/// A 16-value eigenvalue profile. Renders as a sparkline bar chart.
/// Each value is in [0.0, 1.0]. The profile is the spectral signature
/// of a model or document at a given moment.
#[derive(Clone, Debug, PartialEq)]
pub struct EigenvalueProfile {
    pub id: String,
    pub values: [f64; 16],
    pub label: Option<String>,
}

impl EigenvalueProfile {
    pub fn new(id: impl Into<String>, values: [f64; 16]) -> Self {
        EigenvalueProfile { id: id.into(), values, label: None }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Normalize values to [0.0, 1.0].
    pub fn normalize(&self) -> [f64; 16] {
        let max = self.values.iter().cloned().fold(0.0_f64, f64::max);
        if max == 0.0 {
            return [0.0; 16];
        }
        let mut normalized = [0.0; 16];
        for (i, &v) in self.values.iter().enumerate() {
            normalized[i] = v / max;
        }
        normalized
    }
}

impl Addressable for EigenvalueProfile {
    fn oid(&self) -> Oid {
        let values_str: String = self
            .values
            .iter()
            .map(|v| format!("{:.6}", v))
            .collect::<Vec<_>>()
            .join(",");
        Oid::hash(format!("eigenvalue_profile:{}:{}", self.id, values_str).as_bytes())
    }
}

// ---------------------------------------------------------------------------
// LossHeatmap — per-line color map
// ---------------------------------------------------------------------------

/// A loss heatmap. Each entry is a (line_number, loss_value) pair.
/// Renders as a colored bar alongside text content.
#[derive(Clone, Debug, PartialEq)]
pub struct LossHeatmap {
    pub id: String,
    pub entries: Vec<LossEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LossEntry {
    pub line: usize,
    pub loss: f64,
    /// Optional label for this line.
    pub label: Option<String>,
}

impl LossHeatmap {
    pub fn new(id: impl Into<String>) -> Self {
        LossHeatmap { id: id.into(), entries: vec![] }
    }

    pub fn with_entry(mut self, line: usize, loss: f64) -> Self {
        self.entries.push(LossEntry { line, loss, label: None });
        self
    }

    pub fn max_loss(&self) -> f64 {
        self.entries.iter().map(|e| e.loss).fold(0.0_f64, f64::max)
    }
}

impl Addressable for LossHeatmap {
    fn oid(&self) -> Oid {
        let entries_str: String = self
            .entries
            .iter()
            .map(|e| format!("{}:{:.4}", e.line, e.loss))
            .collect::<Vec<_>>()
            .join(",");
        Oid::hash(format!("loss_heatmap:{}:{}", self.id, entries_str).as_bytes())
    }
}

// ---------------------------------------------------------------------------
// MixingFader — slider with eigenvalue visualization
// ---------------------------------------------------------------------------

/// A mixing fader control. Position [0.0, 1.0] interpolates between two states.
/// The fader displays an eigenvalue sparkline showing the current mixture.
#[derive(Clone, Debug, PartialEq)]
pub struct MixingFader {
    pub id: String,
    pub label: String,
    /// Current position [0.0, 1.0]
    pub position: f64,
    /// Profile at position 0.0
    pub profile_low: EigenvalueProfile,
    /// Profile at position 1.0
    pub profile_high: EigenvalueProfile,
}

impl MixingFader {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        low: EigenvalueProfile,
        high: EigenvalueProfile,
    ) -> Self {
        MixingFader {
            id: id.into(),
            label: label.into(),
            position: 0.5,
            profile_low: low,
            profile_high: high,
        }
    }

    /// Compute the interpolated eigenvalue profile at the current position.
    pub fn current_profile(&self) -> [f64; 16] {
        let t = self.position;
        let mut result = [0.0; 16];
        for i in 0..16 {
            result[i] = self.profile_low.values[i] * (1.0 - t)
                + self.profile_high.values[i] * t;
        }
        result
    }
}

impl Addressable for MixingFader {
    fn oid(&self) -> Oid {
        Oid::hash(
            format!(
                "mixing_fader:{}:{}:{:.6}:{}:{}",
                self.id,
                self.label,
                self.position,
                self.profile_low.oid(),
                self.profile_high.oid(),
            )
            .as_bytes(),
        )
    }
}

// ---------------------------------------------------------------------------
// TournamentBracket — 5-model fan-out
// ---------------------------------------------------------------------------

/// A tournament bracket for model comparison.
/// 5 models compete in rounds. Renders as a fan-out bracket visualization.
#[derive(Clone, Debug, PartialEq)]
pub struct TournamentBracket {
    pub id: String,
    pub title: String,
    pub contestants: Vec<Contestant>,
    pub rounds: Vec<Round>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Contestant {
    pub id: String,
    pub name: String,
    pub profile: EigenvalueProfile,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Round {
    pub round_number: usize,
    pub matches: Vec<Match>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    pub left_id: String,
    pub right_id: String,
    pub winner_id: Option<String>,
    pub score: Option<(f64, f64)>,
}

impl TournamentBracket {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        TournamentBracket {
            id: id.into(),
            title: title.into(),
            contestants: vec![],
            rounds: vec![],
        }
    }

    pub fn with_contestant(mut self, c: Contestant) -> Self {
        self.contestants.push(c);
        self
    }
}

impl Addressable for TournamentBracket {
    fn oid(&self) -> Oid {
        let contestant_oids: String = self
            .contestants
            .iter()
            .map(|c| c.profile.oid().to_string())
            .collect::<Vec<_>>()
            .join(",");
        Oid::hash(
            format!("tournament:{}:{}", self.id, contestant_oids).as_bytes(),
        )
    }
}

// ---------------------------------------------------------------------------
// CouplingGraph — node connectivity visualization
// ---------------------------------------------------------------------------

/// A graph of nodes and their coupling strengths.
/// Renders as a force-directed graph or adjacency matrix heatmap.
#[derive(Clone, Debug, PartialEq)]
pub struct CouplingGraph {
    pub id: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    /// Coupling strength [0.0, 1.0].
    pub strength: f64,
}

impl CouplingGraph {
    pub fn new(id: impl Into<String>) -> Self {
        CouplingGraph { id: id.into(), nodes: vec![], edges: vec![] }
    }

    pub fn with_node(mut self, node: GraphNode) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn with_edge(mut self, from: impl Into<String>, to: impl Into<String>, strength: f64) -> Self {
        self.edges.push(GraphEdge { from: from.into(), to: to.into(), strength });
        self
    }
}

impl Addressable for CouplingGraph {
    fn oid(&self) -> Oid {
        let nodes_str: String = self
            .nodes
            .iter()
            .map(|n| format!("{}:{:.4}", n.id, n.weight))
            .collect::<Vec<_>>()
            .join(",");
        let edges_str: String = self
            .edges
            .iter()
            .map(|e| format!("{}->{}:{:.4}", e.from, e.to, e.strength))
            .collect::<Vec<_>>()
            .join(",");
        Oid::hash(
            format!("coupling_graph:{}:{}:{}", self.id, nodes_str, edges_str).as_bytes(),
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_profile(id: &str, value: f64) -> EigenvalueProfile {
        EigenvalueProfile::new(id, [value; 16])
    }

    #[test]
    fn eigenvalue_profile_content_addressed() {
        let a = flat_profile("model_a", 0.5);
        let b = flat_profile("model_a", 0.5);
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn different_values_different_oid() {
        let a = flat_profile("model_a", 0.5);
        let b = flat_profile("model_a", 0.6);
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn eigenvalue_profile_normalize() {
        let p = EigenvalueProfile::new("test", {
            let mut v = [0.0; 16];
            v[0] = 2.0;
            v[1] = 1.0;
            v
        });
        let n = p.normalize();
        assert!((n[0] - 1.0).abs() < 1e-9);
        assert!((n[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn eigenvalue_profile_normalize_zeros() {
        let p = flat_profile("zero", 0.0);
        let n = p.normalize();
        assert!(n.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn loss_heatmap_content_addressed() {
        let a = LossHeatmap::new("heatmap").with_entry(0, 0.1).with_entry(1, 0.5);
        let b = LossHeatmap::new("heatmap").with_entry(0, 0.1).with_entry(1, 0.5);
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn loss_heatmap_different_entries_different_oid() {
        let a = LossHeatmap::new("heatmap").with_entry(0, 0.1);
        let b = LossHeatmap::new("heatmap").with_entry(0, 0.9);
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn loss_heatmap_max_loss() {
        let h = LossHeatmap::new("h").with_entry(0, 0.1).with_entry(1, 0.8).with_entry(2, 0.3);
        assert!((h.max_loss() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn mixing_fader_interpolates() {
        let mut low_vals = [0.0; 16];
        let mut high_vals = [1.0; 16];
        low_vals[0] = 0.0;
        high_vals[0] = 1.0;

        let low = EigenvalueProfile::new("low", low_vals);
        let high = EigenvalueProfile::new("high", high_vals);
        let mut fader = MixingFader::new("f", "Fader", low, high);
        fader.position = 0.5;

        let current = fader.current_profile();
        assert!((current[0] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn mixing_fader_position_zero_is_low() {
        let low = flat_profile("low", 0.0);
        let high = flat_profile("high", 1.0);
        let mut fader = MixingFader::new("f", "Fader", low, high);
        fader.position = 0.0;
        let current = fader.current_profile();
        assert!(current.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn mixing_fader_position_one_is_high() {
        let low = flat_profile("low", 0.0);
        let high = flat_profile("high", 1.0);
        let mut fader = MixingFader::new("f", "Fader", low, high);
        fader.position = 1.0;
        let current = fader.current_profile();
        assert!(current.iter().all(|&v| (v - 1.0).abs() < 1e-9));
    }

    #[test]
    fn mixing_fader_content_addressed() {
        let low = flat_profile("low", 0.0);
        let high = flat_profile("high", 1.0);
        let a = MixingFader::new("f", "Fader", low.clone(), high.clone());
        let b = MixingFader::new("f", "Fader", low, high);
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn tournament_bracket_content_addressed() {
        let c = Contestant {
            id: "m1".into(),
            name: "Model 1".into(),
            profile: flat_profile("m1", 0.5),
        };
        let a = TournamentBracket::new("t", "Tournament").with_contestant(c.clone());
        let b = TournamentBracket::new("t", "Tournament").with_contestant(c);
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn tournament_bracket_different_contestants_different_oid() {
        let c1 = Contestant { id: "m1".into(), name: "M1".into(), profile: flat_profile("m1", 0.5) };
        let c2 = Contestant { id: "m2".into(), name: "M2".into(), profile: flat_profile("m2", 0.7) };
        let a = TournamentBracket::new("t", "T").with_contestant(c1);
        let b = TournamentBracket::new("t", "T").with_contestant(c2);
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn coupling_graph_content_addressed() {
        let a = CouplingGraph::new("g")
            .with_node(GraphNode { id: "a".into(), label: "A".into(), weight: 1.0 })
            .with_edge("a", "b", 0.5);
        let b = CouplingGraph::new("g")
            .with_node(GraphNode { id: "a".into(), label: "A".into(), weight: 1.0 })
            .with_edge("a", "b", 0.5);
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn coupling_graph_different_strength_different_oid() {
        let a = CouplingGraph::new("g").with_edge("a", "b", 0.5);
        let b = CouplingGraph::new("g").with_edge("a", "b", 0.9);
        assert_ne!(a.oid(), b.oid());
    }
}
