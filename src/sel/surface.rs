//! Surface model — NL input to NamedOptic classification.
//!
//! COULD MOVE TO MIRROR: The `NamedOptic` type and the optic vocabulary
//! could move to mirror as compiler primitives. The pattern vocabulary
//! (NL stems) could become a `@surface` boot grammar.
//! STAYS IN SPECTRAL: The classifier weights, training loop, and inference.
//! Mirror proves the grammar is valid. Spectral runs the trained model.
//!
//! ~30K parameters: 100 NL features -> 32 hidden (ReLU) -> 50 operations (softmax).
//! Two matrix multiplies. Linear classifier. Convex — monotonically convergent.

use crate::sel::matrix;

// ---------------------------------------------------------------------------
// Named optic — the output vocabulary
// ---------------------------------------------------------------------------

/// The five prism operations, named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    Focus,
    Project,
    Split,
    Zoom,
    Refract,
}

impl OpKind {
    pub fn count() -> usize {
        5
    }

    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(OpKind::Focus),
            1 => Some(OpKind::Project),
            2 => Some(OpKind::Split),
            3 => Some(OpKind::Zoom),
            4 => Some(OpKind::Refract),
            _ => None,
        }
    }

    pub fn index(&self) -> usize {
        *self as usize
    }

    pub fn name(&self) -> &'static str {
        match self {
            OpKind::Focus => "focus",
            OpKind::Project => "project",
            OpKind::Split => "split",
            OpKind::Zoom => "zoom",
            OpKind::Refract => "refract",
        }
    }
}

/// A classified operation with confidence and extracted arguments.
#[derive(Clone, Debug)]
pub struct NamedOptic {
    pub op: OpKind,
    pub confidence: f64,
    pub args: Vec<String>,
}

// ---------------------------------------------------------------------------
// Feature extraction — NL tokenizer
// ---------------------------------------------------------------------------

/// Number of NL features extracted from input text.
pub const NL_FEATURE_DIM: usize = 100;

/// Number of output classes (operations).
/// 5 base ops x 10 sub-patterns = 50.
pub const OP_DIM: usize = 50;

/// Hidden layer dimension.
pub const HIDDEN_DIM: usize = 32;

/// Total parameter count for Surface model.
pub const SURFACE_PARAM_COUNT: usize =
    NL_FEATURE_DIM * HIDDEN_DIM + HIDDEN_DIM + HIDDEN_DIM * OP_DIM + OP_DIM;

/// NL pattern entry: a word stem and the feature index it activates.
struct Pattern {
    stem: &'static str,
    feature_idx: usize,
}

/// The pattern vocabulary. Each entry maps an NL word stem to a feature index.
/// Feature indices 0-19: Focus patterns
/// Feature indices 20-39: Project patterns
/// Feature indices 40-59: Split patterns
/// Feature indices 60-79: Zoom patterns
/// Feature indices 80-99: Refract patterns
const PATTERNS: &[Pattern] = &[
    // Focus patterns (0-19)
    Pattern { stem: "what", feature_idx: 0 },
    Pattern { stem: "explain", feature_idx: 1 },
    Pattern { stem: "tell", feature_idx: 2 },
    Pattern { stem: "describe", feature_idx: 3 },
    Pattern { stem: "show", feature_idx: 4 },
    Pattern { stem: "look", feature_idx: 5 },
    Pattern { stem: "focus", feature_idx: 6 },
    Pattern { stem: "about", feature_idx: 7 },
    Pattern { stem: "meaning", feature_idx: 8 },
    Pattern { stem: "define", feature_idx: 9 },
    Pattern { stem: "clarify", feature_idx: 10 },
    Pattern { stem: "understand", feature_idx: 11 },
    Pattern { stem: "detail", feature_idx: 12 },
    Pattern { stem: "info", feature_idx: 13 },
    Pattern { stem: "who", feature_idx: 14 },
    Pattern { stem: "where", feature_idx: 15 },
    Pattern { stem: "when", feature_idx: 16 },
    Pattern { stem: "which", feature_idx: 17 },
    Pattern { stem: "how", feature_idx: 18 },
    Pattern { stem: "why", feature_idx: 19 },
    // Project patterns (20-39)
    Pattern { stem: "filter", feature_idx: 20 },
    Pattern { stem: "only", feature_idx: 21 },
    Pattern { stem: "just", feature_idx: 22 },
    Pattern { stem: "select", feature_idx: 23 },
    Pattern { stem: "pick", feature_idx: 24 },
    Pattern { stem: "narrow", feature_idx: 25 },
    Pattern { stem: "specific", feature_idx: 26 },
    Pattern { stem: "exclude", feature_idx: 27 },
    Pattern { stem: "remove", feature_idx: 28 },
    Pattern { stem: "without", feature_idx: 29 },
    Pattern { stem: "except", feature_idx: 30 },
    Pattern { stem: "keep", feature_idx: 31 },
    Pattern { stem: "discard", feature_idx: 32 },
    Pattern { stem: "cut", feature_idx: 33 },
    Pattern { stem: "trim", feature_idx: 34 },
    Pattern { stem: "prune", feature_idx: 35 },
    Pattern { stem: "restrict", feature_idx: 36 },
    Pattern { stem: "limit", feature_idx: 37 },
    Pattern { stem: "constrain", feature_idx: 38 },
    Pattern { stem: "reduce", feature_idx: 39 },
    // Split patterns (40-59)
    Pattern { stem: "connect", feature_idx: 40 },
    Pattern { stem: "link", feature_idx: 41 },
    Pattern { stem: "relat", feature_idx: 42 },
    Pattern { stem: "between", feature_idx: 43 },
    Pattern { stem: "compar", feature_idx: 44 },
    Pattern { stem: "differ", feature_idx: 45 },
    Pattern { stem: "similar", feature_idx: 46 },
    Pattern { stem: "versus", feature_idx: 47 },
    Pattern { stem: "branch", feature_idx: 48 },
    Pattern { stem: "fork", feature_idx: 49 },
    Pattern { stem: "split", feature_idx: 50 },
    Pattern { stem: "divid", feature_idx: 51 },
    Pattern { stem: "partition", feature_idx: 52 },
    Pattern { stem: "group", feature_idx: 53 },
    Pattern { stem: "cluster", feature_idx: 54 },
    Pattern { stem: "categor", feature_idx: 55 },
    Pattern { stem: "map", feature_idx: 56 },
    Pattern { stem: "graph", feature_idx: 57 },
    Pattern { stem: "path", feature_idx: 58 },
    Pattern { stem: "traversal", feature_idx: 59 },
    // Zoom patterns (60-79)
    Pattern { stem: "deeper", feature_idx: 60 },
    Pattern { stem: "zoom", feature_idx: 61 },
    Pattern { stem: "detail", feature_idx: 62 },
    Pattern { stem: "more", feature_idx: 63 },
    Pattern { stem: "expand", feature_idx: 64 },
    Pattern { stem: "elaborat", feature_idx: 65 },
    Pattern { stem: "further", feature_idx: 66 },
    Pattern { stem: "closer", feature_idx: 67 },
    Pattern { stem: "into", feature_idx: 68 },
    Pattern { stem: "inside", feature_idx: 69 },
    Pattern { stem: "within", feature_idx: 70 },
    Pattern { stem: "beneath", feature_idx: 71 },
    Pattern { stem: "under", feature_idx: 72 },
    Pattern { stem: "below", feature_idx: 73 },
    Pattern { stem: "drill", feature_idx: 74 },
    Pattern { stem: "dig", feature_idx: 75 },
    Pattern { stem: "explore", feature_idx: 76 },
    Pattern { stem: "investigat", feature_idx: 77 },
    Pattern { stem: "examin", feature_idx: 78 },
    Pattern { stem: "inspect", feature_idx: 79 },
    // Refract patterns (80-99)
    Pattern { stem: "settle", feature_idx: 80 },
    Pattern { stem: "crystalliz", feature_idx: 81 },
    Pattern { stem: "done", feature_idx: 82 },
    Pattern { stem: "finish", feature_idx: 83 },
    Pattern { stem: "complet", feature_idx: 84 },
    Pattern { stem: "conclud", feature_idx: 85 },
    Pattern { stem: "summar", feature_idx: 86 },
    Pattern { stem: "wrap", feature_idx: 87 },
    Pattern { stem: "final", feature_idx: 88 },
    Pattern { stem: "save", feature_idx: 89 },
    Pattern { stem: "commit", feature_idx: 90 },
    Pattern { stem: "record", feature_idx: 91 },
    Pattern { stem: "store", feature_idx: 92 },
    Pattern { stem: "persist", feature_idx: 93 },
    Pattern { stem: "end", feature_idx: 94 },
    Pattern { stem: "stop", feature_idx: 95 },
    Pattern { stem: "exit", feature_idx: 96 },
    Pattern { stem: "quit", feature_idx: 97 },
    Pattern { stem: "close", feature_idx: 98 },
    Pattern { stem: "enough", feature_idx: 99 },
];

/// Extract NL features from input text.
///
/// Splits into lowercase words, matches against stem patterns.
/// Returns a feature vector of length NL_FEATURE_DIM.
pub fn extract_features(input: &str) -> Vec<f64> {
    let mut features = vec![0.0; NL_FEATURE_DIM];
    let lower = input.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    for word in &words {
        for pattern in PATTERNS {
            if word.starts_with(pattern.stem) || word.contains(pattern.stem) {
                features[pattern.feature_idx] = 1.0;
            }
        }
    }

    features
}

/// Extract argument words — words that aren't pattern stems.
/// These are the graph node references in the query.
fn extract_args(input: &str) -> Vec<String> {
    let lower = input.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    let stop_words = [
        "the", "a", "an", "is", "are", "was", "were", "do", "does", "did",
        "to", "of", "in", "on", "at", "for", "with", "from", "by", "and",
        "or", "not", "no", "it", "its", "this", "that", "i", "me", "my",
        "can", "could", "would", "should", "will", "shall", "may", "might",
        "?", "!", ".", ",",
    ];

    let mut args = Vec::new();
    for word in &words {
        let w = word.trim_matches(|c: char| !c.is_alphanumeric());
        if w.is_empty() {
            continue;
        }
        // Skip if it's a pattern stem
        let is_pattern = PATTERNS.iter().any(|p| w.starts_with(p.stem) || w.contains(p.stem));
        let is_stop = stop_words.contains(&w);
        if !is_pattern && !is_stop {
            args.push(w.to_string());
        }
    }
    args
}

// ---------------------------------------------------------------------------
// Surface model
// ---------------------------------------------------------------------------

/// The Surface model weights.
pub struct Surface {
    /// Layer 1: NL_FEATURE_DIM x HIDDEN_DIM, row-major.
    pub w1: Vec<f64>,
    /// Bias 1: HIDDEN_DIM.
    pub b1: Vec<f64>,
    /// Layer 2: HIDDEN_DIM x OP_DIM, row-major.
    pub w2: Vec<f64>,
    /// Bias 2: OP_DIM.
    pub b2: Vec<f64>,
}

/// Confidence threshold below which Surface returns None (triggers fallback).
pub const CONFIDENCE_THRESHOLD: f64 = 0.15;

impl Surface {
    /// Create an untrained Surface with Xavier-initialized weights.
    pub fn untrained(seed: u64) -> Self {
        let mut rng = matrix::SimpleRng(seed);
        let scale1 = (2.0 / (NL_FEATURE_DIM + HIDDEN_DIM) as f64).sqrt();
        let scale2 = (2.0 / (HIDDEN_DIM + OP_DIM) as f64).sqrt();

        Surface {
            w1: (0..HIDDEN_DIM * NL_FEATURE_DIM)
                .map(|_| rng.next_normal() * scale1)
                .collect(),
            b1: vec![0.0; HIDDEN_DIM],
            w2: (0..OP_DIM * HIDDEN_DIM)
                .map(|_| rng.next_normal() * scale2)
                .collect(),
            b2: vec![0.0; OP_DIM],
        }
    }

    /// Classify NL input into a NamedOptic.
    ///
    /// Returns `Some(NamedOptic)` if confidence >= threshold, `None` otherwise
    /// (triggers Claude fallback).
    pub fn classify(&self, input: &str) -> Option<NamedOptic> {
        let features = extract_features(input);
        let (op_idx, confidence) = self.forward(&features);

        if confidence < CONFIDENCE_THRESHOLD {
            return None;
        }

        // Map the 50-class output back to one of 5 base ops
        // Classes 0-9 = Focus, 10-19 = Project, 20-29 = Split, 30-39 = Zoom, 40-49 = Refract
        let base_op_idx = op_idx / 10;
        let op = OpKind::from_index(base_op_idx)?;
        let args = extract_args(input);

        Some(NamedOptic {
            op,
            confidence,
            args,
        })
    }

    /// Forward pass: features -> (predicted_class_index, confidence).
    fn forward(&self, features: &[f64]) -> (usize, f64) {
        let hidden = matrix::matmul(&self.w1, features, &self.b1, HIDDEN_DIM, NL_FEATURE_DIM);
        let hidden = matrix::relu(&hidden);
        let logits = matrix::matmul(&self.w2, &hidden, &self.b2, OP_DIM, HIDDEN_DIM);
        let probs = matrix::softmax(&logits);
        let best = matrix::argmax(&probs);
        (best, probs[best])
    }

    /// Train on one example: predicted was wrong, actual was right.
    ///
    /// `predicted_op`: what Surface classified.
    /// `actual_op`: what the user actually wanted.
    /// `features`: the NL feature vector from the input.
    /// `learning_rate`: SGD step size.
    pub fn train(&mut self, predicted_op: OpKind, actual_op: OpKind, features: &[f64], learning_rate: f64) {
        if predicted_op == actual_op {
            return; // Correct — no update needed.
        }

        // Forward pass to get hidden activations
        let hidden_raw = matrix::matmul(&self.w1, features, &self.b1, HIDDEN_DIM, NL_FEATURE_DIM);
        let hidden = matrix::relu(&hidden_raw);
        let logits = matrix::matmul(&self.w2, &hidden, &self.b2, OP_DIM, HIDDEN_DIM);
        let probs = matrix::softmax(&logits);

        // Target: the first sub-pattern of the actual op class
        let target_idx = actual_op.index() * 10;
        let d_logits = matrix::cross_entropy_gradient(&probs, target_idx);

        // Gradients for W2, b2
        let grad_w2 = matrix::outer_product(&d_logits, &hidden);
        self.w2 = matrix::matrix_subtract(&self.w2, &matrix::matrix_scale(&grad_w2, learning_rate));
        let scaled_b2 = matrix::matrix_scale(&d_logits, learning_rate);
        self.b2 = matrix::matrix_subtract(&self.b2, &scaled_b2);

        // Backprop through ReLU to hidden layer
        let mut d_hidden = vec![0.0; HIDDEN_DIM];
        for j in 0..HIDDEN_DIM {
            for i in 0..OP_DIM {
                d_hidden[j] += self.w2[i * HIDDEN_DIM + j] * d_logits[i];
            }
            // ReLU derivative: 0 if hidden_raw <= 0, 1 otherwise
            if hidden_raw[j] <= 0.0 {
                d_hidden[j] = 0.0;
            }
        }

        // Gradients for W1, b1
        let grad_w1 = matrix::outer_product(&d_hidden, features);
        self.w1 = matrix::matrix_subtract(&self.w1, &matrix::matrix_scale(&grad_w1, learning_rate));
        let scaled_b1 = matrix::matrix_scale(&d_hidden, learning_rate);
        self.b1 = matrix::matrix_subtract(&self.b1, &scaled_b1);
    }

    /// Get the raw weight vectors for serialization.
    pub fn weights(&self) -> (&[f64], &[f64], &[f64], &[f64]) {
        (&self.w1, &self.b1, &self.w2, &self.b2)
    }

    /// Total parameter count.
    pub fn param_count(&self) -> usize {
        self.w1.len() + self.b1.len() + self.w2.len() + self.b2.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_extraction_focus_patterns() {
        let features = extract_features("what is loss?");
        assert_eq!(features[0], 1.0, "what should activate feature 0");
    }

    #[test]
    fn feature_extraction_split_patterns() {
        let features = extract_features("how does loss connect to growth?");
        assert_eq!(features[40], 1.0, "connect should activate feature 40");
    }

    #[test]
    fn feature_extraction_zoom_patterns() {
        let features = extract_features("go deeper into eigenvalues");
        assert_eq!(features[60], 1.0, "deeper should activate feature 60");
    }

    #[test]
    fn feature_extraction_refract_patterns() {
        let features = extract_features("settle the session");
        assert_eq!(features[80], 1.0, "settle should activate feature 80");
    }

    #[test]
    fn feature_extraction_project_patterns() {
        let features = extract_features("filter only the important nodes");
        assert_eq!(features[20], 1.0, "filter should activate feature 20");
        assert_eq!(features[21], 1.0, "only should activate feature 21");
    }

    #[test]
    fn feature_vector_length() {
        let features = extract_features("anything");
        assert_eq!(features.len(), NL_FEATURE_DIM);
    }

    #[test]
    fn extract_args_filters_stems_and_stops() {
        let args = extract_args("what connects loss to growth?");
        assert!(args.contains(&"loss".to_string()));
        assert!(args.contains(&"growth".to_string()));
        assert!(!args.contains(&"what".to_string()));
        assert!(!args.contains(&"to".to_string()));
    }

    #[test]
    fn surface_param_count() {
        let s = Surface::untrained(42);
        assert_eq!(s.param_count(), SURFACE_PARAM_COUNT);
    }

    #[test]
    fn surface_classify_returns_named_optic() {
        let s = Surface::untrained(42);
        // An untrained model may or may not classify with high confidence,
        // but it should not panic.
        let _ = s.classify("what is loss?");
    }

    #[test]
    fn surface_forward_produces_valid_index() {
        let s = Surface::untrained(42);
        let features = extract_features("what is loss?");
        let (idx, conf) = s.forward(&features);
        assert!(idx < OP_DIM);
        assert!(conf > 0.0);
        assert!(conf <= 1.0);
    }

    #[test]
    fn opkind_roundtrip() {
        for i in 0..5 {
            let op = OpKind::from_index(i).unwrap();
            assert_eq!(op.index(), i);
        }
    }

    #[test]
    fn opkind_names() {
        assert_eq!(OpKind::Focus.name(), "focus");
        assert_eq!(OpKind::Split.name(), "split");
        assert_eq!(OpKind::Refract.name(), "refract");
    }

    #[test]
    fn opkind_out_of_range() {
        assert!(OpKind::from_index(5).is_none());
        assert!(OpKind::from_index(99).is_none());
    }

    #[test]
    fn surface_training_shifts_weights() {
        let mut s = Surface::untrained(42);
        let features = extract_features("what connects loss to growth?");
        let w2_before: Vec<f64> = s.w2.clone();

        // Train: predicted Focus, actual Split
        s.train(OpKind::Focus, OpKind::Split, &features, 0.01);

        // Weights should have changed
        assert_ne!(s.w2, w2_before, "training should modify weights");
    }

    #[test]
    fn surface_training_no_op_when_correct() {
        let mut s = Surface::untrained(42);
        let features = extract_features("what is loss?");
        let w2_before: Vec<f64> = s.w2.clone();

        s.train(OpKind::Focus, OpKind::Focus, &features, 0.01);

        assert_eq!(s.w2, w2_before, "no update when prediction is correct");
    }

    #[test]
    fn surface_repeated_training_converges() {
        let mut s = Surface::untrained(42);

        // Train on "what is X" -> Focus, 50 times
        let features = extract_features("what is loss?");
        for _ in 0..50 {
            s.train(OpKind::Split, OpKind::Focus, &features, 0.05);
        }

        // After training, the model should classify this input as Focus-range
        let (idx, _) = s.forward(&features);
        let base_op = idx / 10;
        assert_eq!(base_op, OpKind::Focus.index(),
            "after repeated training, 'what is X' should classify as Focus, got class {}", idx);
    }

    #[test]
    fn at_least_20_patterns() {
        assert!(PATTERNS.len() >= 20, "must have at least 20 NL patterns, got {}", PATTERNS.len());
    }

    #[test]
    fn all_five_ops_covered_by_patterns() {
        // Focus: 0-19, Project: 20-39, Split: 40-59, Zoom: 60-79, Refract: 80-99
        let mut has_focus = false;
        let mut has_project = false;
        let mut has_split = false;
        let mut has_zoom = false;
        let mut has_refract = false;
        for p in PATTERNS {
            match p.feature_idx {
                0..=19 => has_focus = true,
                20..=39 => has_project = true,
                40..=59 => has_split = true,
                60..=79 => has_zoom = true,
                80..=99 => has_refract = true,
                _ => {}
            }
        }
        assert!(has_focus, "no Focus patterns");
        assert!(has_project, "no Project patterns");
        assert!(has_split, "no Split patterns");
        assert!(has_zoom, "no Zoom patterns");
        assert!(has_refract, "no Refract patterns");
    }
}
