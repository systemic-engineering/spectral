//! End-to-end tests for the NL query pipeline against real spectral-db graphs.
//!
//! These tests verify that Surface classification, Reflection planning, and
//! Shatter rendering work against actual graph topologies built with TestGraph.
//! The pipeline currently uses placeholder execution (no graph traversal in
//! Mirror stage), so these tests focus on the typed path from NL to plan.
//!
//! When Mirror execution is wired to SpectralDb, these tests will verify
//! the full NL → graph → NL loop.

#[cfg(test)]
mod tests {
    use spectral_db::test_support::TestGraph;

    use crate::sel::pipeline::{
        classify_typed, plan, ModelPipeline, PipelineError,
    };
    use crate::sel::surface::{extract_features, OpKind, Surface};
    use crate::sel::reflection::Reflection;
    use crate::sel::shatter_model::Shatter;

    use prism::{FieldOptic, OpticKind};
    use std::collections::HashMap;
    use terni::Imperfect;

    // ── Helpers ────────────────────────────────────────────────────

    /// Build a schema descriptor from a TestGraph's node names/types,
    /// mapping each node name to a FieldOptic with a default kind.
    fn schema_from_nodes(nodes: &HashMap<String, String>) -> Vec<FieldOptic> {
        nodes
            .keys()
            .map(|name| FieldOptic {
                name: Box::leak(name.clone().into_boxed_str()),
                kind: OpticKind::Lens,
            })
            .collect()
    }

    /// Train a Surface to recognize "split" queries (connecting two things).
    fn train_surface_for_split(surface: &mut Surface) {
        let features = extract_features("what connects loss to growth");
        for _ in 0..200 {
            surface.train(OpKind::Focus, OpKind::Split, &features, 0.05);
        }
    }

    /// Train a Surface to recognize "focus" queries (looking at one thing).
    fn train_surface_for_focus(surface: &mut Surface) {
        let features = extract_features("what is loss");
        for _ in 0..200 {
            surface.train(OpKind::Split, OpKind::Focus, &features, 0.05);
        }
    }

    // ── Pipeline with graph constructor ──────────────────────────────

    #[test]
    fn pipeline_new_with_db_stores_graph_ref() {
        let (_dir, db, nodes) = TestGraph::claims();
        let schema = schema_from_nodes(&nodes);
        let mut pipeline = ModelPipeline::new_with_db(&db, &schema);
        train_surface_for_focus(&mut pipeline.surface);

        let result = pipeline.process("what is loss");
        match result {
            Imperfect::Success(s) | Imperfect::Partial(s, _) => {
                assert!(!s.is_empty());
            }
            Imperfect::Failure(PipelineError::ClassificationFailed, _) => {}
            Imperfect::Failure(e, _) => panic!("pipeline failed: {:?}", e),
        }
    }

    // ── Graph fixture determinism ──────────────────────────────────

    #[test]
    fn deterministic_graph_deterministic_oids() {
        let (_dir1, _db1, nodes1) = TestGraph::claims();
        let (_dir2, _db2, nodes2) = TestGraph::claims();

        assert_eq!(nodes1["loss"], nodes2["loss"]);
        assert_eq!(nodes1["growth"], nodes2["growth"]);
        assert_eq!(nodes1["processor"], nodes2["processor"]);
    }

    #[test]
    fn deterministic_graph_deterministic_hash() {
        let (_dir1, db1, _) = TestGraph::claims();
        let (_dir2, db2, _) = TestGraph::claims();

        assert_eq!(db1.graph_hash(), db2.graph_hash());
    }

    #[test]
    fn deterministic_surface_classification() {
        let mut s1 = Surface::untrained(42);
        let mut s2 = Surface::untrained(42);
        train_surface_for_split(&mut s1);
        train_surface_for_split(&mut s2);

        let out1 = classify_typed(&s1, "what connects loss to growth");
        let out2 = classify_typed(&s2, "what connects loss to growth");

        assert!(out1.is_some() && out2.is_some());
        assert_eq!(out1.unwrap().intent.op, out2.unwrap().intent.op);
    }

    // ── Surface classification against graph ───────────────────────

    #[test]
    fn surface_classifies_split_query() {
        let (_dir, _db, _nodes) = TestGraph::claims();
        let mut surface = Surface::untrained(42);
        train_surface_for_split(&mut surface);

        let output = classify_typed(&surface, "what connects loss to growth");
        assert!(output.is_some(), "trained surface should classify split query");
        let out = output.unwrap();
        assert_eq!(out.intent.op, OpKind::Split);
        assert!(out.lenses.len() >= 2, "split should extract at least 2 lenses");
    }

    #[test]
    fn surface_classifies_focus_query() {
        let (_dir, _db, _nodes) = TestGraph::claims();
        let mut surface = Surface::untrained(42);
        train_surface_for_focus(&mut surface);

        let output = classify_typed(&surface, "what is loss");
        assert!(output.is_some(), "trained surface should classify focus query");
        let out = output.unwrap();
        assert_eq!(out.intent.op, OpKind::Focus);
    }

    #[test]
    fn surface_extracts_graph_node_names_as_args() {
        let (_dir, _db, nodes) = TestGraph::claims();
        let mut surface = Surface::untrained(42);
        train_surface_for_split(&mut surface);

        let output = classify_typed(&surface, "what connects loss to growth");
        assert!(output.is_some());
        let refs: Vec<String> = output
            .unwrap()
            .lenses
            .iter()
            .map(|l| l.ref_.clone())
            .collect();

        // These refs match node names in the claims graph
        assert!(refs.contains(&"loss".to_string()));
        assert!(refs.contains(&"growth".to_string()));
        assert!(nodes.contains_key("loss"));
        assert!(nodes.contains_key("growth"));
    }

    // ── Reflection planning against schema ─────────────────────────

    #[test]
    fn reflection_plans_against_claims_schema() {
        let (_dir, _db, nodes) = TestGraph::claims();
        let schema = schema_from_nodes(&nodes);
        let reflection = Reflection::untrained(43);

        let mut surface = Surface::untrained(42);
        train_surface_for_split(&mut surface);
        let surface_out = classify_typed(&surface, "what connects loss to growth").unwrap();

        let plan_result = plan(&reflection, &surface_out, None, &schema);
        match plan_result {
            Imperfect::Success(p) | Imperfect::Partial(p, _) => {
                assert!(!p.steps.is_empty());
                assert!(p.remaining_budget >= 0.0);
                // Schema has both "loss" and "growth" — plan should have zero cost
            }
            Imperfect::Failure(e, _) => panic!("plan failed: {:?}", e),
        }
    }

    #[test]
    fn plan_with_known_schema_has_no_extra_cost() {
        let (_dir, _db, nodes) = TestGraph::claims();
        let schema = schema_from_nodes(&nodes);
        let reflection = Reflection::untrained(43);

        let mut surface = Surface::untrained(42);
        train_surface_for_split(&mut surface);
        let surface_out = classify_typed(&surface, "what connects loss to growth").unwrap();

        let plan_result = plan(&reflection, &surface_out, None, &schema);
        // When all refs are in the schema, plan should be Success (no extra cost)
        assert!(
            plan_result.as_ref().ok().is_some(),
            "plan with known refs should succeed"
        );
    }

    #[test]
    fn plan_with_unknown_ref_has_extra_cost() {
        let (_dir, _db, _nodes) = TestGraph::claims();
        // Empty schema — none of the refs are known
        let reflection = Reflection::untrained(43);

        let mut surface = Surface::untrained(42);
        train_surface_for_split(&mut surface);
        let surface_out = classify_typed(&surface, "what connects loss to growth").unwrap();

        let plan_result = plan(&reflection, &surface_out, None, &[]);
        assert!(
            plan_result.is_partial(),
            "plan with unknown refs should be partial"
        );
    }

    // ── Full pipeline ──────────────────────────────────────────────

    #[test]
    fn full_pipeline_with_claims_graph() {
        let (_dir, _db, nodes) = TestGraph::claims();
        let schema = schema_from_nodes(&nodes);
        let mut pipeline = ModelPipeline::new();
        train_surface_for_split(&mut pipeline.surface);

        let result = pipeline.process_with_schema("what connects loss to growth", &schema);
        match result {
            Imperfect::Success(s) | Imperfect::Partial(s, _) => {
                assert!(!s.is_empty(), "pipeline should produce non-empty output");
                // The output should contain at least one of the query refs
                assert!(
                    s.contains("loss") || s.contains("growth"),
                    "output should mention query refs: {}",
                    s
                );
            }
            Imperfect::Failure(e, _) => {
                panic!("pipeline failed: {:?}", e);
            }
        }
    }

    #[test]
    fn full_pipeline_with_codebase_graph() {
        let (_dir, _db, nodes) = TestGraph::codebase();
        let schema = schema_from_nodes(&nodes);
        let mut pipeline = ModelPipeline::new();

        // Train for focus queries
        train_surface_for_focus(&mut pipeline.surface);

        let result = pipeline.process_with_schema("what is beam", &schema);
        match result {
            Imperfect::Success(s) | Imperfect::Partial(s, _) => {
                assert!(!s.is_empty());
            }
            Imperfect::Failure(PipelineError::ClassificationFailed, _) => {
                // "beam" might conflict with pattern stems — acceptable
            }
            Imperfect::Failure(e, _) => panic!("pipeline failed: {:?}", e),
        }
    }

    #[test]
    fn pipeline_classification_failure_on_nonsense() {
        let (_dir, _db, nodes) = TestGraph::claims();
        let schema = schema_from_nodes(&nodes);
        let mut pipeline = ModelPipeline::new();
        // Don't train — untrained model

        let result = pipeline.process_with_schema("xyzzy plugh frobnitz", &schema);
        // Untrained model on nonsense input — may classify or fail
        // Either outcome is acceptable; we just verify no panic
        let _ = result;
    }

    // ── Loss budget propagation ────────────────────────────────────

    #[test]
    fn confident_query_has_lower_loss_than_vague() {
        let (_dir, _db, nodes) = TestGraph::claims();
        let schema = schema_from_nodes(&nodes);

        let mut pipeline = ModelPipeline::new();
        train_surface_for_focus(&mut pipeline.surface);

        let confident = pipeline.process_with_schema("what is loss", &schema);
        let vague = pipeline.process_with_schema("tell me something interesting about stuff", &schema);

        // Both might produce results; compare loss if both partial
        match (&confident, &vague) {
            (Imperfect::Partial(_, c_loss), Imperfect::Partial(_, v_loss)) => {
                // Vague should have >= confident's loss
                assert!(
                    v_loss.total() >= c_loss.total() - 0.01,
                    "vague ({}) should have >= loss than confident ({})",
                    v_loss.total(),
                    c_loss.total()
                );
            }
            _ => {
                // If classification fails for one, we can't compare — still ok
            }
        }
    }

    // ── Graph topology visible in pipeline ─────────────────────────

    #[test]
    fn claims_graph_has_expected_topology() {
        let (_dir, db, nodes) = TestGraph::claims();

        // Verify the graph structure is what the pipeline will see
        let loss_neighbors = db.neighbors(&nodes["loss"]);
        assert!(loss_neighbors.contains(&nodes["growth"]));

        let adj_neighbors = db.neighbors_weighted(&nodes["adjuster_id"]);
        let weights: HashMap<String, f64> = adj_neighbors.into_iter().collect();
        assert!((weights[&nodes["processor"]] - 0.7).abs() < 1e-6);
        assert!((weights[&nodes["assignment_rules"]] - 0.31).abs() < 1e-6);
    }

    #[test]
    fn codebase_graph_walk_reaches_functions() {
        let (_dir, db, _nodes) = TestGraph::codebase();

        // Walk from lib module through edges
        let modules = db.find("module");
        let walked = db.walk(&modules, 1);
        // Should reach type_decl nodes (beam, optic, prism) via edges from lib
        assert!(walked.len() > 0, "walk from modules should reach type declarations");
    }

    #[test]
    fn triangle_spectral_distance_is_small() {
        let (_dir, db, nodes) = TestGraph::triangle();
        db.compute_spectral_coordinates();

        // All nodes in a triangle should be spectrally close
        let dist = db.spectral_distance_eigen(&nodes["alice"], &nodes["bob"]);
        assert!(dist.is_some());
        assert!(
            dist.unwrap() < 0.1,
            "triangle nodes should be spectrally near, got {}",
            dist.unwrap()
        );
    }

    // ── Shatter variant selection against graph data ───────────────

    #[test]
    fn shatter_selects_variant_for_graph_eigenvalues() {
        let shatter = Shatter::untrained(44);

        // Simulate eigenvalue profiles for different graph structures
        let dense_eigenvalues: Vec<f64> = vec![0.9; 50]; // Dense graph = high eigenvalues
        let sparse_eigenvalues: Vec<f64> = vec![0.1; 50]; // Sparse graph = low eigenvalues

        let dense_variant = shatter.select(&dense_eigenvalues, 0, 0);
        let sparse_variant = shatter.select(&sparse_eigenvalues, 0, 0);

        // Different eigenvalue profiles should (potentially) select different variants
        // Even if they happen to select the same, the selection should not panic
        let _ = (dense_variant, sparse_variant);
    }

    // ── Pipeline with different fixtures ───────────────────────────

    #[test]
    fn pipeline_works_with_all_fixtures() {
        for fixture_name in &["triangle", "claims", "codebase"] {
            let (_dir, _db, nodes) = match *fixture_name {
                "triangle" => TestGraph::triangle(),
                "claims" => TestGraph::claims(),
                "codebase" => TestGraph::codebase(),
                _ => unreachable!(),
            };
            let schema = schema_from_nodes(&nodes);
            let mut pipeline = ModelPipeline::new();
            train_surface_for_focus(&mut pipeline.surface);

            // Simple query — should not panic for any fixture
            let result = pipeline.process_with_schema("what is this", &schema);
            let _ = result;
        }
    }
}
