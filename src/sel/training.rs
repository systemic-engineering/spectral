//! Training pipeline — the tick/tock loop for NL models.
//!
//! COULD MOVE TO MIRROR: The tick/tock structure could become a
//! `@training` boot grammar describing the pipeline shape.
//! STAYS IN SPECTRAL: The actual training execution and weight updates.
//! Mirror defines the pipeline grammar. Spectral executes it.
//!
//! tick:
//!   Surface.classify(input) -> NamedOptic
//!   [mirror navigates graph]
//!   Shatter.render(result, eigenvalues) -> NL text
//!
//! tock:
//!   observe engagement signal
//!   Surface.train(predicted_op, actual_op, features)
//!   Shatter.train(predicted_variant, actual_variant, eigenvalues)
//!   Reflection.update(conversation_state) -> gestalt delta
//!   write Crystal<WeightState> to .shatter file

use crate::sel::reflection::{ConversationState, Reflection};
use crate::sel::shatter_model::{Shatter, Variant};
use crate::sel::surface::{extract_features, NamedOptic, OpKind, Surface};
use crate::sel::weight_file::{WeightLuminosity, WeightState};

// ---------------------------------------------------------------------------
// Training pipeline state
// ---------------------------------------------------------------------------

/// The complete NL pipeline: three models + weight state.
pub struct NLPipeline {
    pub surface: Surface,
    pub shatter: Shatter,
    pub reflection: Reflection,
    pub eigenvalues: Vec<f64>,
    pub turns: u64,
    pub weight_path: Option<String>,
}

/// Result of a tick: what Surface classified and what Shatter rendered.
#[derive(Clone, Debug)]
pub struct TickResult {
    /// The classified optic (None = low confidence, needs fallback).
    pub optic: Option<NamedOptic>,
    /// The rendered text (None = no template matched).
    pub rendered: Option<String>,
    /// The features extracted from input (for training).
    pub features: Vec<f64>,
    /// The variant selected by Shatter.
    pub variant: Option<Variant>,
}

impl NLPipeline {
    /// Create a new pipeline with untrained models.
    pub fn new(seed: u64) -> Self {
        NLPipeline {
            surface: Surface::untrained(seed),
            shatter: Shatter::untrained(seed + 1),
            reflection: Reflection::untrained(seed + 2),
            eigenvalues: vec![0.5; 50],
            turns: 0,
            weight_path: None,
        }
    }

    /// Create a pipeline and load weights from disk if available.
    pub fn with_weights(seed: u64, path: &str) -> Self {
        let mut pipeline = Self::new(seed);
        pipeline.weight_path = Some(path.to_string());

        if let Ok(ws) = WeightState::load(path) {
            ws.apply_to_models(
                &mut pipeline.surface,
                &mut pipeline.shatter,
                &mut pipeline.reflection,
            );
            pipeline.eigenvalues = ws.eigenvalues;
            pipeline.turns = ws.turns;
        }

        pipeline
    }

    /// Tick: process NL input through the pipeline.
    ///
    /// 1. Surface classifies NL input into NamedOptic.
    /// 2. (Caller navigates graph with the optic.)
    /// 3. Shatter renders result with concept name and slots.
    pub fn tick(&self, input: &str, concept: Option<&str>, slots: &[&str]) -> TickResult {
        let features = extract_features(input);
        let optic = self.surface.classify(input);

        let (rendered, variant) = if let Some(concept_name) = concept {
            let v = self.shatter.select(&self.eigenvalues, 0, 0);
            let text = self.shatter.render(&self.eigenvalues, concept_name, slots);
            (text, Some(v))
        } else {
            (None, None)
        };

        TickResult {
            optic,
            rendered,
            features,
            variant,
        }
    }

    /// Tock: update models from engagement signal.
    ///
    /// `tick_result`: the result from the tick phase.
    /// `actual_op`: what the user actually wanted (if known from re-query/correction).
    /// `actual_variant`: what rendering the user preferred (if known).
    /// `conversation_state`: current conversation state for Reflection.
    /// `learning_rate`: SGD step size.
    pub fn tock(
        &mut self,
        tick_result: &TickResult,
        actual_op: Option<OpKind>,
        actual_variant: Option<Variant>,
        conversation_state: &ConversationState,
        learning_rate: f64,
    ) {
        self.turns += 1;

        // Train Surface if we know the correct op
        if let (Some(ref optic), Some(actual)) = (&tick_result.optic, actual_op) {
            self.surface.train(optic.op, actual, &tick_result.features, learning_rate);
        }

        // Train Shatter if we know the correct variant
        if let (Some(predicted), Some(actual)) = (tick_result.variant, actual_variant) {
            self.shatter.train(
                &self.eigenvalues, 0, 0,
                predicted, actual,
                learning_rate,
            );
        }

        // Reflection: observe and update eigenvalues
        let delta = self.reflection.observe(conversation_state);
        self.eigenvalues = Reflection::apply_delta(&self.eigenvalues, &delta, 0.1);
    }

    /// Save current weights to disk.
    pub fn save_weights(&self) -> Result<(), String> {
        let path = self.weight_path.as_ref()
            .ok_or_else(|| "no weight path set".to_string())?;
        let mut ws = WeightState::from_models(&self.surface, &self.shatter, &self.reflection);
        ws.eigenvalues = self.eigenvalues.clone();
        ws.turns = self.turns;
        ws.luminosity = self.compute_luminosity();
        ws.save(path)
    }

    /// Compute luminosity from training state.
    fn compute_luminosity(&self) -> WeightLuminosity {
        if self.turns == 0 {
            WeightLuminosity::Dark
        } else if self.turns < 10 {
            let holonomy = 1.0 - (self.turns as f64 / 10.0);
            WeightLuminosity::Dimmed(holonomy)
        } else {
            WeightLuminosity::Light
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sel::reflection::EngagementSignal;

    fn sample_conversation_state() -> ConversationState {
        ConversationState {
            topic_indices: vec![0, 3],
            engagement: EngagementSignal {
                dwelled: true,
                requeried: false,
                follow_up: true,
                clarification: false,
                topic_change: false,
                response_length: 20,
            },
            turn: 1,
            op_counts: [1, 0, 0, 0, 0],
            prev_eigenvalues: vec![0.5; 8],
        }
    }

    #[test]
    fn pipeline_tick_classifies() {
        let pipeline = NLPipeline::new(42);
        let result = pipeline.tick("what is loss?", None, &[]);
        assert_eq!(result.features.len(), 100);
        // optic may or may not be Some depending on confidence
    }

    #[test]
    fn pipeline_tick_with_concept() {
        use crate::sel::shatter_model::Template;
        let mut pipeline = NLPipeline::new(42);
        pipeline.shatter.templates.push(Template {
            concept: "inverse".to_string(),
            variants: [
                "{0} and {1} are inverses.".to_string(),
                "{0} and {1} feel opposite.".to_string(),
                "{0} is to {1} as up is to down.".to_string(),
                "e.g. {0}=70% means {1}=30%.".to_string(),
                "{0} = 100% - {1}.".to_string(),
            ],
        });

        let result = pipeline.tick("what connects loss to growth?", Some("inverse"), &["loss", "growth"]);
        assert!(result.rendered.is_some());
        assert!(result.variant.is_some());
    }

    #[test]
    fn pipeline_tock_updates_turns() {
        let mut pipeline = NLPipeline::new(42);
        let result = pipeline.tick("what is loss?", None, &[]);
        let state = sample_conversation_state();

        assert_eq!(pipeline.turns, 0);
        pipeline.tock(&result, None, None, &state, 0.01);
        assert_eq!(pipeline.turns, 1);
    }

    #[test]
    fn pipeline_tock_updates_eigenvalues() {
        let mut pipeline = NLPipeline::new(42);
        let result = pipeline.tick("what is loss?", None, &[]);
        let state = sample_conversation_state();
        let eigen_before = pipeline.eigenvalues.clone();

        pipeline.tock(&result, None, None, &state, 0.01);

        // Eigenvalues should have shifted (Reflection applied delta)
        assert_ne!(pipeline.eigenvalues, eigen_before);
    }

    #[test]
    fn pipeline_tock_trains_surface() {
        let mut pipeline = NLPipeline::new(42);
        let result = pipeline.tick("what is loss?", None, &[]);
        let state = sample_conversation_state();
        let w2_before = pipeline.surface.w2.clone();

        // If Surface classified something, training with a different actual should shift weights
        if result.optic.is_some() {
            pipeline.tock(&result, Some(OpKind::Split), None, &state, 0.01);
            // Weights may or may not change depending on predicted op
        } else {
            pipeline.tock(&result, Some(OpKind::Focus), None, &state, 0.01);
            // No optic = no Surface training
            assert_eq!(pipeline.surface.w2, w2_before);
        }
    }

    #[test]
    fn pipeline_luminosity_progression() {
        let mut pipeline = NLPipeline::new(42);

        assert_eq!(pipeline.compute_luminosity(), WeightLuminosity::Dark);

        pipeline.turns = 5;
        match pipeline.compute_luminosity() {
            WeightLuminosity::Dimmed(h) => assert!(h > 0.0 && h < 1.0),
            other => panic!("expected Dimmed, got {:?}", other),
        }

        pipeline.turns = 10;
        assert_eq!(pipeline.compute_luminosity(), WeightLuminosity::Light);
    }

    #[test]
    fn pipeline_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.shatter");
        let path_str = path.to_str().unwrap();

        let mut pipeline = NLPipeline::new(42);
        pipeline.weight_path = Some(path_str.to_string());
        pipeline.turns = 7;
        pipeline.eigenvalues = vec![0.3, 0.7, 0.5];

        pipeline.save_weights().expect("save failed");

        let pipeline2 = NLPipeline::with_weights(42, path_str);
        assert_eq!(pipeline2.turns, 7);
        assert_eq!(pipeline2.eigenvalues, vec![0.3, 0.7, 0.5]);
    }

    #[test]
    fn pipeline_with_missing_weights_uses_defaults() {
        let pipeline = NLPipeline::with_weights(42, "/nonexistent/path.shatter");
        assert_eq!(pipeline.turns, 0);
        assert_eq!(pipeline.eigenvalues.len(), 50);
    }

    #[test]
    fn pipeline_full_tick_tock_cycle() {
        let mut pipeline = NLPipeline::new(42);
        let state = sample_conversation_state();

        // Tick
        let result = pipeline.tick("what is loss?", None, &[]);

        // Tock
        pipeline.tock(&result, Some(OpKind::Focus), None, &state, 0.01);

        assert_eq!(pipeline.turns, 1);
    }

    #[test]
    fn pipeline_multiple_cycles() {
        let mut pipeline = NLPipeline::new(42);

        for i in 0..5 {
            let state = ConversationState {
                topic_indices: vec![i % 10],
                engagement: EngagementSignal {
                    dwelled: true,
                    requeried: false,
                    follow_up: i % 2 == 0,
                    clarification: false,
                    topic_change: false,
                    response_length: 10 + i * 5,
                },
                turn: i,
                op_counts: [i, 0, 0, 0, 0],
                prev_eigenvalues: pipeline.eigenvalues.iter().take(8).cloned().collect(),
            };

            let result = pipeline.tick("focus on eigenvalues", None, &[]);
            pipeline.tock(&result, Some(OpKind::Focus), None, &state, 0.01);
        }

        assert_eq!(pipeline.turns, 5);
    }
}
