//! Integration tests for the NL pipeline.
//! These tests verify the end-to-end flow: Surface → Shatter → Reflection → WeightFile.

#[cfg(test)]
mod tests {
    use crate::sel::training::NLPipeline;
    use crate::sel::surface::OpKind;
    use crate::sel::shatter_model::{Template, Variant};
    use crate::sel::reflection::{ConversationState, EngagementSignal};
    use crate::sel::weight_file::WeightState;

    #[test]
    fn surface_classifies_nl_input() {
        let pipeline = NLPipeline::new(42);
        let result = pipeline.tick("what connects loss to growth?", None, &[]);
        // Surface should produce a classification (may be None if low confidence)
        assert_eq!(result.features.len(), 100);
    }

    #[test]
    fn shatter_renders_personalized_response() {
        let mut pipeline = NLPipeline::new(42);
        pipeline.shatter.templates.push(Template {
            concept: "inverse".to_string(),
            variants: [
                "{0} and {1} are mathematical inverses.".to_string(),
                "{0} and {1} feel like mirror images.".to_string(),
                "{0} is to {1} as day is to night.".to_string(),
                "For example, if {0} = 70%, then {1} = 30%.".to_string(),
                "{0} = 100% - {1}.".to_string(),
            ],
        });

        let result = pipeline.tick(
            "what connects loss to growth?",
            Some("inverse"),
            &["loss", "growth"],
        );
        let text = result.rendered.expect("should render with template");
        assert!(text.contains("loss"), "rendered text should contain 'loss': {}", text);
        assert!(text.contains("growth"), "rendered text should contain 'growth': {}", text);
    }

    #[test]
    fn training_updates_weights() {
        let mut pipeline = NLPipeline::new(42);
        let features = crate::sel::surface::extract_features("what connects loss to growth?");
        let w2_before = pipeline.surface.w2.clone();

        pipeline.surface.train(OpKind::Split, OpKind::Focus, &features, 0.01);

        assert_ne!(pipeline.surface.w2, w2_before, "training should shift weights");
    }

    #[test]
    fn weight_file_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("integration.shatter");
        let path_str = path.to_str().unwrap();

        let mut pipeline = NLPipeline::new(42);
        pipeline.weight_path = Some(path_str.to_string());
        pipeline.turns = 42;
        pipeline.eigenvalues = vec![0.1, 0.2, 0.3];

        pipeline.save_weights().expect("save failed");

        let pipeline2 = NLPipeline::with_weights(42, path_str);
        assert_eq!(pipeline2.turns, 42);
        assert_eq!(pipeline2.eigenvalues, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn full_tick_tock_with_reflection() {
        let mut pipeline = NLPipeline::new(42);
        let state = ConversationState {
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
        };

        let eigen_before = pipeline.eigenvalues.clone();
        let result = pipeline.tick("what is loss?", None, &[]);
        pipeline.tock(&result, Some(OpKind::Focus), None, &state, 0.01);

        assert_eq!(pipeline.turns, 1);
        assert_ne!(pipeline.eigenvalues, eigen_before, "eigenvalues should update after tock");
    }
}
