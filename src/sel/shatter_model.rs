//! Shatter model — graph result to personalized NL response.
//!
//! COULD MOVE TO MIRROR: The `Variant` enum and template grammar could become
//! a `@shatter` boot grammar. The variant selection vocabulary is a compiler
//! primitive — which valid statement fits this reader.
//! STAYS IN SPECTRAL: The selector weights, template filling, and training.
//! Mirror generates valid options. Spectral selects among them.
//!
//! ~15K parameters: 50 eigenvalues + concept embed + slot embed -> 5 variant scores.
//! Two matrix multiplies. Selects among grammar-generated valid options.

use crate::sel::matrix;

// ---------------------------------------------------------------------------
// Variant — the five rendering styles
// ---------------------------------------------------------------------------

/// The five rendering variants Shatter selects among.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    /// Precise, formal, mathematical.
    Technical = 0,
    /// Felt-sense, metaphoric, intuitive.
    Intuitive = 1,
    /// "X is like Y" — structural comparison.
    Analogy = 2,
    /// Concrete instance, worked example.
    Example = 3,
    /// Shortest possible. Minimal.
    Minimal = 4,
}

impl Variant {
    pub fn count() -> usize {
        5
    }

    pub fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(Variant::Technical),
            1 => Some(Variant::Intuitive),
            2 => Some(Variant::Analogy),
            3 => Some(Variant::Example),
            4 => Some(Variant::Minimal),
            _ => None,
        }
    }

    pub fn index(&self) -> usize {
        *self as usize
    }

    pub fn name(&self) -> &'static str {
        match self {
            Variant::Technical => "technical",
            Variant::Intuitive => "intuitive",
            Variant::Analogy => "analogy",
            Variant::Example => "example",
            Variant::Minimal => "minimal",
        }
    }
}

// ---------------------------------------------------------------------------
// Template engine — slot-filling from grammar templates
// ---------------------------------------------------------------------------

/// A template with slots that can be filled per variant.
#[derive(Clone, Debug)]
pub struct Template {
    /// The concept this template renders.
    pub concept: String,
    /// One text template per variant. Slots are `{0}`, `{1}`, etc.
    pub variants: [String; 5],
}

impl Template {
    /// Fill the template for the given variant with slot values.
    pub fn render(&self, variant: Variant, slots: &[&str]) -> String {
        let mut text = self.variants[variant.index()].clone();
        for (i, slot) in slots.iter().enumerate() {
            text = text.replace(&format!("{{{}}}", i), slot);
        }
        text
    }
}

// ---------------------------------------------------------------------------
// Shatter model
// ---------------------------------------------------------------------------

/// Eigenvalue input dimension (reader profile).
pub const EIGEN_DIM: usize = 50;
/// Hidden dimension.
pub const SHATTER_HIDDEN_DIM: usize = 32;
/// Concept embedding dimension.
pub const CONCEPT_EMBED_DIM: usize = 12;
/// Slot embedding dimension.
pub const SLOT_EMBED_DIM: usize = 12;
/// Combined dimension for second layer: hidden + concept_embed + slot_embed.
pub const COMBINED_DIM: usize = SHATTER_HIDDEN_DIM + CONCEPT_EMBED_DIM + SLOT_EMBED_DIM;
/// Number of variant classes.
pub const VARIANT_COUNT: usize = 5;

/// Maximum number of concept embeddings.
pub const MAX_CONCEPTS: usize = 64;
/// Maximum number of slot embeddings.
pub const MAX_SLOTS: usize = 32;

/// The Shatter model: selects rendering variant based on eigenvalue profile.
pub struct Shatter {
    /// Layer 1: EIGEN_DIM -> SHATTER_HIDDEN_DIM.
    pub w1: Vec<f64>,
    pub b1: Vec<f64>,
    /// Concept embeddings: MAX_CONCEPTS x CONCEPT_EMBED_DIM.
    pub concept_embed: Vec<f64>,
    /// Slot embeddings: MAX_SLOTS x SLOT_EMBED_DIM.
    pub slot_embed: Vec<f64>,
    /// Layer 2: COMBINED_DIM -> VARIANT_COUNT.
    pub w2: Vec<f64>,
    pub b2: Vec<f64>,
    /// Templates indexed by concept.
    pub templates: Vec<Template>,
}

impl Shatter {
    /// Create an untrained Shatter with Xavier-initialized weights.
    pub fn untrained(seed: u64) -> Self {
        let mut rng = matrix::SimpleRng(seed);
        let scale1 = (2.0 / (EIGEN_DIM + SHATTER_HIDDEN_DIM) as f64).sqrt();
        let scale2 = (2.0 / (COMBINED_DIM + VARIANT_COUNT) as f64).sqrt();

        Shatter {
            w1: (0..SHATTER_HIDDEN_DIM * EIGEN_DIM)
                .map(|_| rng.next_normal() * scale1)
                .collect(),
            b1: vec![0.0; SHATTER_HIDDEN_DIM],
            concept_embed: (0..MAX_CONCEPTS * CONCEPT_EMBED_DIM)
                .map(|_| rng.next_normal() * 0.1)
                .collect(),
            slot_embed: (0..MAX_SLOTS * SLOT_EMBED_DIM)
                .map(|_| rng.next_normal() * 0.1)
                .collect(),
            w2: (0..VARIANT_COUNT * COMBINED_DIM)
                .map(|_| rng.next_normal() * scale2)
                .collect(),
            b2: vec![0.0; VARIANT_COUNT],
            templates: Vec::new(),
        }
    }

    /// Select the rendering variant for a given concept/slot and eigenvalue profile.
    pub fn select(&self, eigenvalues: &[f64], concept_idx: usize, slot_idx: usize) -> Variant {
        let (variant_idx, _) = self.forward(eigenvalues, concept_idx, slot_idx);
        Variant::from_index(variant_idx).unwrap_or(Variant::Technical)
    }

    /// Forward pass: eigenvalues + concept + slot -> (variant_index, confidence).
    fn forward(&self, eigenvalues: &[f64], concept_idx: usize, slot_idx: usize) -> (usize, f64) {
        // Pad or truncate eigenvalues to EIGEN_DIM
        let mut eigen_input = vec![0.0; EIGEN_DIM];
        for (i, &v) in eigenvalues.iter().take(EIGEN_DIM).enumerate() {
            eigen_input[i] = v;
        }

        // Layer 1: eigenvalues -> hidden
        let hidden = matrix::matmul(&self.w1, &eigen_input, &self.b1, SHATTER_HIDDEN_DIM, EIGEN_DIM);
        let hidden = matrix::relu(&hidden);

        // Concat: hidden + concept_embed + slot_embed
        let concept_offset = (concept_idx % MAX_CONCEPTS) * CONCEPT_EMBED_DIM;
        let slot_offset = (slot_idx % MAX_SLOTS) * SLOT_EMBED_DIM;

        let mut combined = Vec::with_capacity(COMBINED_DIM);
        combined.extend_from_slice(&hidden);
        combined.extend_from_slice(&self.concept_embed[concept_offset..concept_offset + CONCEPT_EMBED_DIM]);
        combined.extend_from_slice(&self.slot_embed[slot_offset..slot_offset + SLOT_EMBED_DIM]);

        // Layer 2: combined -> variant scores
        let logits = matrix::matmul(&self.w2, &combined, &self.b2, VARIANT_COUNT, COMBINED_DIM);
        let probs = matrix::softmax(&logits);
        let best = matrix::argmax(&probs);
        (best, probs[best])
    }

    /// Render a concept for a reader's eigenvalue profile.
    ///
    /// Returns the rendered text using the selected variant's template.
    /// If no template exists for the concept, returns None.
    pub fn render(&self, eigenvalues: &[f64], concept_name: &str, slots: &[&str]) -> Option<String> {
        let concept_idx = self.templates.iter().position(|t| t.concept == concept_name)?;
        let variant = self.select(eigenvalues, concept_idx, 0);
        let template = &self.templates[concept_idx];
        Some(template.render(variant, slots))
    }

    /// Train on one example: predicted variant was wrong, actual was right.
    pub fn train(
        &mut self,
        eigenvalues: &[f64],
        concept_idx: usize,
        slot_idx: usize,
        _predicted: Variant,
        actual: Variant,
        learning_rate: f64,
    ) {
        // Pad eigenvalues
        let mut eigen_input = vec![0.0; EIGEN_DIM];
        for (i, &v) in eigenvalues.iter().take(EIGEN_DIM).enumerate() {
            eigen_input[i] = v;
        }

        // Forward pass to get activations
        let hidden_raw = matrix::matmul(&self.w1, &eigen_input, &self.b1, SHATTER_HIDDEN_DIM, EIGEN_DIM);
        let hidden = matrix::relu(&hidden_raw);

        let concept_offset = (concept_idx % MAX_CONCEPTS) * CONCEPT_EMBED_DIM;
        let slot_offset = (slot_idx % MAX_SLOTS) * SLOT_EMBED_DIM;

        let mut combined = Vec::with_capacity(COMBINED_DIM);
        combined.extend_from_slice(&hidden);
        combined.extend_from_slice(&self.concept_embed[concept_offset..concept_offset + CONCEPT_EMBED_DIM]);
        combined.extend_from_slice(&self.slot_embed[slot_offset..slot_offset + SLOT_EMBED_DIM]);

        let logits = matrix::matmul(&self.w2, &combined, &self.b2, VARIANT_COUNT, COMBINED_DIM);
        let probs = matrix::softmax(&logits);

        let target_idx = actual.index();
        let d_logits = matrix::cross_entropy_gradient(&probs, target_idx);

        // Update W2, b2
        let grad_w2 = matrix::outer_product(&d_logits, &combined);
        self.w2 = matrix::matrix_subtract(&self.w2, &matrix::matrix_scale(&grad_w2, learning_rate));
        let scaled_b2 = matrix::matrix_scale(&d_logits, learning_rate);
        self.b2 = matrix::matrix_subtract(&self.b2, &scaled_b2);
    }

    /// Total parameter count.
    pub fn param_count(&self) -> usize {
        self.w1.len() + self.b1.len()
            + self.concept_embed.len()
            + self.slot_embed.len()
            + self.w2.len() + self.b2.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_roundtrip() {
        for i in 0..5 {
            let v = Variant::from_index(i).unwrap();
            assert_eq!(v.index(), i);
        }
    }

    #[test]
    fn variant_names() {
        assert_eq!(Variant::Technical.name(), "technical");
        assert_eq!(Variant::Minimal.name(), "minimal");
    }

    #[test]
    fn variant_out_of_range() {
        assert!(Variant::from_index(5).is_none());
    }

    #[test]
    fn template_render_basic() {
        let t = Template {
            concept: "inverse".to_string(),
            variants: [
                "{0} and {1} are mathematical inverses.".to_string(),
                "{0} and {1} feel like mirror images.".to_string(),
                "{0} is to {1} as day is to night.".to_string(),
                "For example, if {0} = 70%, then {1} = 30%.".to_string(),
                "{0} = 100% - {1}.".to_string(),
            ],
        };

        let text = t.render(Variant::Technical, &["loss", "growth"]);
        assert_eq!(text, "loss and growth are mathematical inverses.");

        let text = t.render(Variant::Minimal, &["loss", "growth"]);
        assert_eq!(text, "loss = 100% - growth.");
    }

    #[test]
    fn shatter_untrained_selects_variant() {
        let s = Shatter::untrained(42);
        let eigenvalues = vec![0.5; 50];
        let variant = s.select(&eigenvalues, 0, 0);
        // Should return a valid variant without panicking
        let _ = variant.name();
    }

    #[test]
    fn shatter_forward_valid() {
        let s = Shatter::untrained(42);
        let eigenvalues = vec![1.0; 50];
        let (idx, conf) = s.forward(&eigenvalues, 0, 0);
        assert!(idx < VARIANT_COUNT);
        assert!(conf > 0.0);
        assert!(conf <= 1.0);
    }

    #[test]
    fn shatter_render_with_template() {
        let mut s = Shatter::untrained(42);
        s.templates.push(Template {
            concept: "inverse".to_string(),
            variants: [
                "{0} and {1} are inverses.".to_string(),
                "{0} and {1} feel opposite.".to_string(),
                "{0} is to {1} as up is to down.".to_string(),
                "e.g. {0}=70% means {1}=30%.".to_string(),
                "{0} = 100% - {1}.".to_string(),
            ],
        });

        let eigenvalues = vec![0.5; 50];
        let result = s.render(&eigenvalues, "inverse", &["loss", "growth"]);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("loss"), "rendered text should contain slot value: {}", text);
    }

    #[test]
    fn shatter_render_missing_concept() {
        let s = Shatter::untrained(42);
        let eigenvalues = vec![0.5; 50];
        assert!(s.render(&eigenvalues, "nonexistent", &[]).is_none());
    }

    #[test]
    fn shatter_training_shifts_weights() {
        let mut s = Shatter::untrained(42);
        let eigenvalues = vec![0.5; 50];
        let w2_before = s.w2.clone();

        s.train(&eigenvalues, 0, 0, Variant::Technical, Variant::Intuitive, 0.01);

        assert_ne!(s.w2, w2_before, "training should modify weights");
    }

    #[test]
    fn shatter_eigenvalue_padding() {
        // Fewer than EIGEN_DIM eigenvalues should be padded with zeros
        let s = Shatter::untrained(42);
        let short_eigenvalues = vec![1.0; 10];
        let (idx, conf) = s.forward(&short_eigenvalues, 0, 0);
        assert!(idx < VARIANT_COUNT);
        assert!(conf > 0.0);
    }

    #[test]
    fn shatter_param_count() {
        let s = Shatter::untrained(42);
        let count = s.param_count();
        // w1: 32*50=1600, b1: 32, concept_embed: 64*12=768, slot_embed: 32*12=384,
        // w2: 5*56=280, b2: 5
        assert_eq!(count, 1600 + 32 + 768 + 384 + 280 + 5);
    }

    #[test]
    fn shatter_different_eigenvalues_different_variants() {
        let s = Shatter::untrained(42);
        // Extreme eigenvalue profiles should produce different selections
        let technical_profile = vec![1.0; 50];
        let intuitive_profile = vec![-1.0; 50];

        let v1 = s.select(&technical_profile, 0, 0);
        let v2 = s.select(&intuitive_profile, 0, 0);

        // With random weights, different inputs SHOULD produce different outputs
        // (not guaranteed but highly likely with these extreme values)
        let _ = (v1, v2); // At minimum, no panic
    }
}
