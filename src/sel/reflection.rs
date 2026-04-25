//! Reflection model — conversation observation to gestalt update.
//!
//! COULD MOVE TO MIRROR: The GestaltProfile type and AttentionSignature
//! already live in mirror (gestalt.rs). The observation vocabulary
//! (what counts as engagement) could become a `@reflection` boot grammar.
//! STAYS IN SPECTRAL: The observation model weights, training loop, and
//! the process that watches the conversation and updates eigenvalues.
//! Mirror defines what a gestalt IS. Spectral learns how it changes.
//!
//! ~15K parameters: conversation features -> gestalt delta.
//! Observes conversation flow, updates eigenvalue profile.
//! Writes the .shatter weight file (Crystal<WeightState>).

use crate::sel::matrix;

// ---------------------------------------------------------------------------
// Conversation features — what Reflection observes
// ---------------------------------------------------------------------------

/// Dimension of conversation feature vector.
pub const CONV_FEATURE_DIM: usize = 40;

/// Dimension of gestalt delta output (eigenvalue adjustments).
pub const GESTALT_DIM: usize = 50;

/// Hidden dimension.
pub const REFLECTION_HIDDEN_DIM: usize = 32;

/// Engagement signal extracted from conversation flow.
#[derive(Clone, Debug)]
pub struct EngagementSignal {
    /// Did the user dwell on the response (> 3 seconds)?
    pub dwelled: bool,
    /// Did the user re-query the same topic?
    pub requeried: bool,
    /// Did the user ask a follow-up (deeper question)?
    pub follow_up: bool,
    /// Did the user ask for clarification ("what?", "explain simpler")?
    pub clarification: bool,
    /// Did the user change topic entirely?
    pub topic_change: bool,
    /// Number of words in the user's response.
    pub response_length: usize,
}

impl EngagementSignal {
    /// Convert to a feature sub-vector (first 6 features).
    pub fn to_features(&self) -> Vec<f64> {
        vec![
            if self.dwelled { 1.0 } else { 0.0 },
            if self.requeried { 1.0 } else { 0.0 },
            if self.follow_up { 1.0 } else { 0.0 },
            if self.clarification { 1.0 } else { 0.0 },
            if self.topic_change { 1.0 } else { 0.0 },
            (self.response_length as f64).min(100.0) / 100.0,
        ]
    }
}

/// Conversation state observed by Reflection.
#[derive(Clone, Debug)]
pub struct ConversationState {
    /// Topics mentioned (as feature indices into eigenvalue space).
    pub topic_indices: Vec<usize>,
    /// Current engagement signal.
    pub engagement: EngagementSignal,
    /// Turn number in the conversation.
    pub turn: usize,
    /// Operations requested so far (counts).
    pub op_counts: [usize; 5], // focus, project, split, zoom, refract
    /// Previous gestalt eigenvalues.
    pub prev_eigenvalues: Vec<f64>,
}

impl ConversationState {
    /// Extract a fixed-length feature vector for the model.
    pub fn to_features(&self) -> Vec<f64> {
        let mut features = vec![0.0; CONV_FEATURE_DIM];

        // Features 0-5: engagement signal
        let eng = self.engagement.to_features();
        for (i, &v) in eng.iter().enumerate() {
            features[i] = v;
        }

        // Features 6-10: operation counts (normalized)
        for (i, &count) in self.op_counts.iter().enumerate() {
            features[6 + i] = (count as f64).min(20.0) / 20.0;
        }

        // Features 11: turn number (normalized)
        features[11] = (self.turn as f64).min(100.0) / 100.0;

        // Features 12-31: topic activation (which eigenvalue dimensions are active)
        for &idx in &self.topic_indices {
            if idx + 12 < 32 {
                features[12 + idx] = 1.0;
            }
        }

        // Features 32-39: previous eigenvalue summary (top 8 values)
        for (i, &v) in self.prev_eigenvalues.iter().take(8).enumerate() {
            features[32 + i] = v;
        }

        features
    }
}

// ---------------------------------------------------------------------------
// Reflection model
// ---------------------------------------------------------------------------

/// The Reflection model: conversation features -> gestalt delta.
pub struct Reflection {
    /// Layer 1: CONV_FEATURE_DIM -> REFLECTION_HIDDEN_DIM.
    pub w1: Vec<f64>,
    pub b1: Vec<f64>,
    /// Layer 2: REFLECTION_HIDDEN_DIM -> GESTALT_DIM.
    pub w2: Vec<f64>,
    pub b2: Vec<f64>,
}

impl Reflection {
    /// Create an untrained Reflection with Xavier-initialized weights.
    pub fn untrained(seed: u64) -> Self {
        let mut rng = matrix::SimpleRng(seed);
        let scale1 = (2.0 / (CONV_FEATURE_DIM + REFLECTION_HIDDEN_DIM) as f64).sqrt();
        let scale2 = (2.0 / (REFLECTION_HIDDEN_DIM + GESTALT_DIM) as f64).sqrt();

        Reflection {
            w1: (0..REFLECTION_HIDDEN_DIM * CONV_FEATURE_DIM)
                .map(|_| rng.next_normal() * scale1)
                .collect(),
            b1: vec![0.0; REFLECTION_HIDDEN_DIM],
            w2: (0..GESTALT_DIM * REFLECTION_HIDDEN_DIM)
                .map(|_| rng.next_normal() * scale2)
                .collect(),
            b2: vec![0.0; GESTALT_DIM],
        }
    }

    /// Observe a conversation turn and compute gestalt delta.
    ///
    /// Returns a vector of eigenvalue adjustments (positive = comprehension improved,
    /// negative = comprehension decreased or confusion detected).
    pub fn observe(&self, state: &ConversationState) -> Vec<f64> {
        let features = state.to_features();
        self.forward(&features)
    }

    /// Forward pass: conversation features -> gestalt delta.
    fn forward(&self, features: &[f64]) -> Vec<f64> {
        let hidden = matrix::matmul(
            &self.w1, features, &self.b1,
            REFLECTION_HIDDEN_DIM, CONV_FEATURE_DIM,
        );
        let hidden = matrix::relu(&hidden);

        // Output is tanh-bounded delta (between -1 and 1 per dimension)
        let raw = matrix::matmul(
            &self.w2, &hidden, &self.b2,
            GESTALT_DIM, REFLECTION_HIDDEN_DIM,
        );

        // Tanh activation for bounded output
        raw.iter().map(|&x| x.tanh()).collect()
    }

    /// Apply the gestalt delta to eigenvalues.
    ///
    /// `eigenvalues`: current reader eigenvalues.
    /// `delta`: output from `observe()`.
    /// `alpha`: blending factor (0.0 = no change, 1.0 = full delta).
    ///
    /// Returns updated eigenvalues.
    pub fn apply_delta(eigenvalues: &[f64], delta: &[f64], alpha: f64) -> Vec<f64> {
        let mut updated = eigenvalues.to_vec();
        // Ensure same length
        while updated.len() < delta.len() {
            updated.push(0.0);
        }
        for (i, d) in delta.iter().enumerate() {
            if i < updated.len() {
                updated[i] += alpha * d;
                // Clamp to [0, 1]
                updated[i] = updated[i].clamp(0.0, 1.0);
            }
        }
        updated
    }

    /// Train: the delta between predicted engagement and actual engagement.
    ///
    /// `predicted_delta`: what Reflection predicted (from `observe()`).
    /// `actual_delta`: what actually happened (measured from next turn's engagement).
    /// `features`: the conversation feature vector.
    /// `learning_rate`: SGD step size.
    pub fn train(
        &mut self,
        predicted_delta: &[f64],
        actual_delta: &[f64],
        features: &[f64],
        learning_rate: f64,
    ) {
        // MSE gradient: d/dw = 2 * (predicted - actual) * d_predicted/dw
        // For simplicity, we use the output error directly.
        let error: Vec<f64> = predicted_delta.iter()
            .zip(actual_delta.iter())
            .map(|(&p, &a)| p - a)
            .collect();

        // Forward pass for intermediate activations
        let hidden_raw = matrix::matmul(
            &self.w1, features, &self.b1,
            REFLECTION_HIDDEN_DIM, CONV_FEATURE_DIM,
        );
        let hidden = matrix::relu(&hidden_raw);

        // Gradient for W2: error * hidden^T (simplified — ignoring tanh derivative for stability)
        let grad_w2 = matrix::outer_product(&error, &hidden);
        self.w2 = matrix::matrix_subtract(&self.w2, &matrix::matrix_scale(&grad_w2, learning_rate));
        let scaled_b2 = matrix::matrix_scale(&error, learning_rate);
        self.b2 = matrix::matrix_subtract(&self.b2, &scaled_b2);

        // Backprop through hidden layer
        let mut d_hidden = vec![0.0; REFLECTION_HIDDEN_DIM];
        for j in 0..REFLECTION_HIDDEN_DIM {
            for i in 0..GESTALT_DIM {
                d_hidden[j] += self.w2[i * REFLECTION_HIDDEN_DIM + j] * error[i];
            }
            if hidden_raw[j] <= 0.0 {
                d_hidden[j] = 0.0;
            }
        }

        let grad_w1 = matrix::outer_product(&d_hidden, features);
        self.w1 = matrix::matrix_subtract(&self.w1, &matrix::matrix_scale(&grad_w1, learning_rate));
        let scaled_b1 = matrix::matrix_scale(&d_hidden, learning_rate);
        self.b1 = matrix::matrix_subtract(&self.b1, &scaled_b1);
    }

    /// Total parameter count.
    pub fn param_count(&self) -> usize {
        self.w1.len() + self.b1.len() + self.w2.len() + self.b2.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_engagement() -> EngagementSignal {
        EngagementSignal {
            dwelled: true,
            requeried: false,
            follow_up: true,
            clarification: false,
            topic_change: false,
            response_length: 25,
        }
    }

    fn sample_state() -> ConversationState {
        ConversationState {
            topic_indices: vec![0, 3, 7],
            engagement: sample_engagement(),
            turn: 5,
            op_counts: [3, 1, 0, 2, 0],
            prev_eigenvalues: vec![0.5; 8],
        }
    }

    #[test]
    fn engagement_to_features() {
        let eng = sample_engagement();
        let features = eng.to_features();
        assert_eq!(features.len(), 6);
        assert_eq!(features[0], 1.0); // dwelled
        assert_eq!(features[1], 0.0); // not requeried
        assert_eq!(features[2], 1.0); // follow_up
    }

    #[test]
    fn conversation_state_features_length() {
        let state = sample_state();
        let features = state.to_features();
        assert_eq!(features.len(), CONV_FEATURE_DIM);
    }

    #[test]
    fn conversation_state_topic_activation() {
        let state = sample_state();
        let features = state.to_features();
        // Topic indices 0, 3, 7 -> features 12, 15, 19
        assert_eq!(features[12], 1.0);
        assert_eq!(features[15], 1.0);
        assert_eq!(features[19], 1.0);
        // Unmentioned topics should be 0
        assert_eq!(features[13], 0.0);
    }

    #[test]
    fn reflection_observe_produces_delta() {
        let r = Reflection::untrained(42);
        let state = sample_state();
        let delta = r.observe(&state);
        assert_eq!(delta.len(), GESTALT_DIM);
        // All values should be in [-1, 1] (tanh output)
        for &d in &delta {
            assert!(d >= -1.0 && d <= 1.0, "delta out of range: {}", d);
        }
    }

    #[test]
    fn reflection_apply_delta_clamps() {
        let eigenvalues = vec![0.9; 5];
        let delta = vec![0.5, -2.0, 0.0, 0.3, -0.1];
        let updated = Reflection::apply_delta(&eigenvalues, &delta, 1.0);
        assert_eq!(updated.len(), 5);
        assert_eq!(updated[0], 1.0); // clamped to 1.0
        assert_eq!(updated[1], 0.0); // clamped to 0.0
        assert!((updated[2] - 0.9).abs() < 1e-10); // unchanged
    }

    #[test]
    fn reflection_apply_delta_alpha_zero() {
        let eigenvalues = vec![0.5; 5];
        let delta = vec![1.0; 5];
        let updated = Reflection::apply_delta(&eigenvalues, &delta, 0.0);
        for (i, &v) in updated.iter().enumerate() {
            assert!((v - 0.5).abs() < 1e-10, "eigenvalue {} changed with alpha=0", i);
        }
    }

    #[test]
    fn reflection_apply_delta_pads_short_eigenvalues() {
        let eigenvalues = vec![0.5; 3];
        let delta = vec![0.1; 5];
        let updated = Reflection::apply_delta(&eigenvalues, &delta, 1.0);
        assert_eq!(updated.len(), 5);
    }

    #[test]
    fn reflection_training_shifts_weights() {
        let mut r = Reflection::untrained(42);
        let state = sample_state();
        let features = state.to_features();
        let predicted = r.observe(&state);
        let actual = vec![0.0; GESTALT_DIM]; // actual: no change

        let w2_before = r.w2.clone();
        r.train(&predicted, &actual, &features, 0.01);

        assert_ne!(r.w2, w2_before, "training should modify weights");
    }

    #[test]
    fn reflection_param_count() {
        let r = Reflection::untrained(42);
        let count = r.param_count();
        // w1: 32*40=1280, b1: 32, w2: 50*32=1600, b2: 50
        assert_eq!(count, 1280 + 32 + 1600 + 50);
    }

    #[test]
    fn reflection_deterministic() {
        let r = Reflection::untrained(42);
        let state = sample_state();
        let d1 = r.observe(&state);
        let d2 = r.observe(&state);
        assert_eq!(d1, d2);
    }
}
