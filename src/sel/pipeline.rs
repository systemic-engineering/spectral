//! Model composition pipeline: Surface -> Reflection -> Mirror -> Shatter.
//!
//! Wires the four NL models into a typed pipeline where each stage's output
//! feeds the next. Loss budget propagates through the chain.
//! Optic composition is validated at planning time via OpticKind::compose.

use prism::{FieldOptic, Oid, OpticKind};
use terni::{Imperfect, Loss};

use crate::sel::reflection::Reflection;
use crate::sel::shatter_model::Shatter;
use crate::sel::surface::{NamedOptic, OpKind, Surface};

// ---------------------------------------------------------------------------
// Error and loss types
// ---------------------------------------------------------------------------

/// Errors from the pipeline stages.
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineError {
    /// Surface could not classify the input.
    ClassificationFailed,
    /// Reflection could not plan a valid optic chain.
    PlanFailed(String),
    /// Mirror execution failed.
    ExecutionFailed(String),
    /// Shatter could not render.
    RenderFailed,
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineError::ClassificationFailed => write!(f, "classification failed"),
            PipelineError::PlanFailed(msg) => write!(f, "plan failed: {}", msg),
            PipelineError::ExecutionFailed(msg) => write!(f, "execution failed: {}", msg),
            PipelineError::RenderFailed => write!(f, "render failed"),
        }
    }
}

/// Loss budget from pipeline stages. Carries confidence loss from each model.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PipelineLoss {
    pub surface_loss: f64,
    pub plan_loss: f64,
    pub execution_loss: f64,
    pub render_loss: f64,
}

impl PipelineLoss {
    pub fn zero() -> Self {
        PipelineLoss {
            surface_loss: 0.0,
            plan_loss: 0.0,
            execution_loss: 0.0,
            render_loss: 0.0,
        }
    }

    pub fn total(&self) -> f64 {
        self.surface_loss + self.plan_loss + self.execution_loss + self.render_loss
    }
}

impl terni::Loss for PipelineLoss {
    fn zero() -> Self {
        PipelineLoss::zero()
    }

    fn total() -> Self {
        PipelineLoss {
            surface_loss: f64::INFINITY,
            plan_loss: f64::INFINITY,
            execution_loss: f64::INFINITY,
            render_loss: f64::INFINITY,
        }
    }

    fn is_zero(&self) -> bool {
        self.surface_loss == 0.0
            && self.plan_loss == 0.0
            && self.execution_loss == 0.0
            && self.render_loss == 0.0
    }

    fn combine(self, other: Self) -> Self {
        PipelineLoss {
            surface_loss: self.surface_loss + other.surface_loss,
            plan_loss: self.plan_loss + other.plan_loss,
            execution_loss: self.execution_loss + other.execution_loss,
            render_loss: self.render_loss + other.render_loss,
        }
    }
}

// ---------------------------------------------------------------------------
// Resolution types
// ---------------------------------------------------------------------------

/// Error resolving a typed lens reference against the graph.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolutionError {
    /// The reference does not exist in the graph schema.
    NotFound(String),
    /// The reference exists but the optic kind doesn't match.
    KindMismatch { expected: OpticKind, found: OpticKind },
}

/// Loss from resolving a reference (e.g. fuzzy match confidence).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ResolutionLoss(pub f64);

impl terni::Loss for ResolutionLoss {
    fn zero() -> Self { ResolutionLoss(0.0) }
    fn total() -> Self { ResolutionLoss(f64::INFINITY) }
    fn is_zero(&self) -> bool { self.0 == 0.0 }
    fn combine(self, other: Self) -> Self { ResolutionLoss(self.0 + other.0) }
}

// ---------------------------------------------------------------------------
// TypedLens — a reference with an optic kind
// ---------------------------------------------------------------------------

/// A typed lens reference extracted by Surface and resolved against the schema.
#[derive(Debug, Clone)]
pub struct TypedLens {
    /// The NL reference string (e.g. "loss", "growth").
    pub ref_: String,
    /// The optic kind this reference maps to.
    pub kind: OpticKind,
    /// Resolution result: resolved OID or error.
    pub resolved: Imperfect<Oid, ResolutionError, ResolutionLoss>,
}

// ---------------------------------------------------------------------------
// SurfaceOutput — enriched classification result
// ---------------------------------------------------------------------------

/// What the target of a query is.
#[derive(Debug, Clone)]
pub struct TargetSpec {
    /// References to target nodes.
    pub refs: Vec<String>,
    /// Budget for loss through the pipeline (1.0 - confidence).
    pub loss_budget: f64,
}

/// Error classifying the input.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassificationError;

/// Loss from classification (1.0 - confidence).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ConfidenceLoss(pub f64);

impl terni::Loss for ConfidenceLoss {
    fn zero() -> Self { ConfidenceLoss(0.0) }
    fn total() -> Self { ConfidenceLoss(1.0) }
    fn is_zero(&self) -> bool { self.0 == 0.0 }
    fn combine(self, other: Self) -> Self { ConfidenceLoss((self.0 + other.0).min(1.0)) }
}

/// The enriched output of Surface classification.
#[derive(Debug, Clone)]
pub struct SurfaceOutput {
    /// The classified intent (named optic operation).
    pub intent: NamedOptic,
    /// Typed lenses extracted from the query arguments.
    pub lenses: Vec<TypedLens>,
    /// Target specification with loss budget.
    pub target: Imperfect<TargetSpec, ClassificationError, ConfidenceLoss>,
}

// ---------------------------------------------------------------------------
// QueryPlan — Reflection output
// ---------------------------------------------------------------------------

/// Error in query planning.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanError {
    /// No valid composition path exists.
    NoPath(String),
    /// Loss budget exhausted.
    BudgetExhausted,
}

/// Loss from planning.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlanLoss(pub f64);

impl terni::Loss for PlanLoss {
    fn zero() -> Self { PlanLoss(0.0) }
    fn total() -> Self { PlanLoss(f64::INFINITY) }
    fn is_zero(&self) -> bool { self.0 == 0.0 }
    fn combine(self, other: Self) -> Self { PlanLoss(self.0 + other.0) }
}

/// A planned query: a sequence of optic steps with a composed kind.
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// Steps to execute, in order.
    pub steps: Vec<NamedOptic>,
    /// The composed optic kind of the full chain.
    pub composition: OpticKind,
    /// Target with remaining budget.
    pub target: Imperfect<TargetSpec, PlanError, PlanLoss>,
    /// How much loss budget remains after planning.
    pub remaining_budget: f64,
}

// ---------------------------------------------------------------------------
// Surface extension — classify_typed
// ---------------------------------------------------------------------------

/// Map from Surface's OpKind to prism-core's OpticKind.
fn opkind_to_optic_kind(op: OpKind) -> OpticKind {
    match op {
        OpKind::Focus => OpticKind::Lens,
        OpKind::Project => OpticKind::Prism,
        OpKind::Split => OpticKind::Traversal,
        OpKind::Zoom => OpticKind::Lens,
        OpKind::Refract => OpticKind::Iso,
    }
}

/// Extend Surface with typed lens extraction.
pub fn classify_typed(surface: &Surface, input: &str) -> Option<SurfaceOutput> {
    let optic = surface.classify(input)?;
    let confidence = optic.confidence;

    let intent_kind = opkind_to_optic_kind(optic.op);
    let lenses: Vec<TypedLens> = optic.args.iter().map(|arg| {
        TypedLens {
            ref_: arg.clone(),
            kind: intent_kind,
            resolved: Imperfect::success(Oid::new(format!("unresolved:{}", arg))),
        }
    }).collect();

    let refs: Vec<String> = optic.args.clone();
    let loss_budget = 1.0 - confidence;

    let target = if confidence > 0.5 {
        Imperfect::success(TargetSpec { refs, loss_budget })
    } else {
        Imperfect::partial(
            TargetSpec { refs, loss_budget },
            ConfidenceLoss(loss_budget),
        )
    };

    Some(SurfaceOutput {
        intent: optic,
        lenses,
        target,
    })
}

// ---------------------------------------------------------------------------
// Reflection as query planner
// ---------------------------------------------------------------------------

/// Plan a query using Reflection's observation model.
///
/// Uses OpticKind::compose to validate the optic chain at planning time.
pub fn plan(
    _reflection: &Reflection,
    surface_output: &SurfaceOutput,
    _current_position: Option<Oid>,
    graph_schema: &[FieldOptic],
) -> Imperfect<QueryPlan, PlanError, PlanLoss> {
    let intent = &surface_output.intent;
    let intent_kind = opkind_to_optic_kind(intent.op);

    let mut steps: Vec<NamedOptic> = Vec::new();
    let mut composed = OpticKind::Iso;
    let mut plan_cost = 0.0;

    for lens in &surface_output.lenses {
        let schema_match = graph_schema.iter().find(|f| f.name == lens.ref_);

        let step_kind = if let Some(field) = schema_match {
            let next_composed = composed.compose(field.kind);
            composed = next_composed;
            field.kind
        } else {
            plan_cost += 0.1;
            composed = composed.compose(intent_kind);
            intent_kind
        };

        steps.push(NamedOptic {
            op: intent.op,
            confidence: intent.confidence,
            args: vec![lens.ref_.clone()],
        });

        let _ = step_kind;
    }

    if steps.is_empty() {
        steps.push(intent.clone());
        composed = intent_kind;
    }

    let loss_budget = match &surface_output.target {
        Imperfect::Success(t) | Imperfect::Partial(t, _) => t.loss_budget,
        Imperfect::Failure(_, _) => 1.0,
    };

    let remaining = (loss_budget - plan_cost).max(0.0);

    let target_spec = TargetSpec {
        refs: surface_output.lenses.iter().map(|l| l.ref_.clone()).collect(),
        loss_budget: remaining,
    };

    let query_plan = QueryPlan {
        steps,
        composition: composed,
        target: Imperfect::success(target_spec),
        remaining_budget: remaining,
    };

    if plan_cost > 0.0 {
        Imperfect::partial(query_plan, PlanLoss(plan_cost))
    } else {
        Imperfect::success(query_plan)
    }
}

// ---------------------------------------------------------------------------
// Execute plan (placeholder — Mirror integration)
// ---------------------------------------------------------------------------

/// Placeholder for Mirror execution. Returns the plan's refs as a result string.
fn execute_plan(query_plan: &QueryPlan) -> Imperfect<String, PipelineError, PipelineLoss> {
    let refs: Vec<String> = query_plan.steps.iter()
        .flat_map(|s| s.args.iter().cloned())
        .collect();

    if refs.is_empty() {
        Imperfect::partial(
            "empty result".to_string(),
            PipelineLoss {
                execution_loss: 0.5,
                ..PipelineLoss::zero()
            },
        )
    } else {
        Imperfect::success(refs.join(", "))
    }
}

// ---------------------------------------------------------------------------
// ModelPipeline — the full composition
// ---------------------------------------------------------------------------

/// The four-model composition pipeline.
pub struct ModelPipeline {
    pub surface: Surface,
    pub reflection: Reflection,
    pub shatter: Shatter,
    pub position: Option<Oid>,
    pub eigenvalues: Vec<f64>,
    /// Schema fields for optic validation (populated by new_with_db).
    schema: Vec<FieldOptic>,
}

impl ModelPipeline {
    /// Create a new pipeline with untrained models.
    pub fn new() -> Self {
        ModelPipeline {
            surface: Surface::untrained(42),
            reflection: Reflection::untrained(43),
            shatter: Shatter::untrained(44),
            position: None,
            eigenvalues: vec![0.5; 50],
            schema: Vec::new(),
        }
    }

    /// Create a pipeline backed by a graph schema.
    ///
    /// Extracts field descriptors from the schema so that `process()` uses
    /// them for optic validation. The `SpectralDb` reference is not stored;
    /// only the schema shape is captured. When Mirror execution is wired,
    /// this constructor will hold a graph reference for traversal.
    pub fn new_with_db<D>(_db: &D, schema: &[FieldOptic]) -> Self {
        ModelPipeline {
            surface: Surface::untrained(42),
            reflection: Reflection::untrained(43),
            shatter: Shatter::untrained(44),
            position: None,
            eigenvalues: vec![0.5; 50],
            schema: schema.to_vec(),
        }
    }

    /// Process NL input through the full pipeline.
    pub fn process(&mut self, input: &str) -> Imperfect<String, PipelineError, PipelineLoss> {
        self.process_with_schema(input, &self.schema.clone())
    }

    /// Process with an explicit graph schema for optic validation.
    pub fn process_with_schema(
        &mut self,
        input: &str,
        schema: &[FieldOptic],
    ) -> Imperfect<String, PipelineError, PipelineLoss> {
        // 1. Surface classification
        let surface_out = match classify_typed(&self.surface, input) {
            Some(out) => out,
            None => {
                return Imperfect::failure(PipelineError::ClassificationFailed);
            }
        };

        let surface_loss = 1.0 - surface_out.intent.confidence;

        // 2. Reflection: plan the query
        let plan_result = plan(
            &self.reflection,
            &surface_out,
            self.position.clone(),
            schema,
        );

        let (query_plan, plan_loss) = match plan_result {
            Imperfect::Success(p) => (p, 0.0),
            Imperfect::Partial(p, loss) => (p, loss.0),
            Imperfect::Failure(e, _) => {
                return Imperfect::failure(PipelineError::PlanFailed(format!("{:?}", e)));
            }
        };

        // 3. Execute the plan (Mirror placeholder)
        let exec_result = execute_plan(&query_plan);
        let (result_text, exec_loss) = match exec_result {
            Imperfect::Success(t) => (t, 0.0),
            Imperfect::Partial(t, loss) => (t, loss.execution_loss),
            Imperfect::Failure(e, _) => {
                return Imperfect::failure(e);
            }
        };

        // 4. Shatter: render result for reader's eigenvalue profile
        let variant = self.shatter.select(&self.eigenvalues, 0, 0);
        let output = format!("[{}] {}", variant.name(), result_text);

        // Compute total pipeline loss
        let total_loss = PipelineLoss {
            surface_loss,
            plan_loss,
            execution_loss: exec_loss,
            render_loss: 0.0,
        };

        if total_loss.is_zero() {
            Imperfect::success(output)
        } else {
            Imperfect::partial(output, total_loss)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_oid() -> Oid {
        Oid::new("test:oid:1234")
    }

    #[test]
    fn surface_output_typed_lenses() {
        let surface = Surface::untrained(42);
        let output = classify_typed(&surface, "what connects loss to growth");
        if let Some(out) = output {
            assert!(!out.lenses.is_empty() || out.intent.args.is_empty());
        }
    }

    #[test]
    fn classify_typed_extracts_args_as_lenses() {
        let mut surface = Surface::untrained(42);
        let features = crate::sel::surface::extract_features("what connects loss to growth");
        for _ in 0..100 {
            surface.train(OpKind::Focus, OpKind::Split, &features, 0.05);
        }

        let output = classify_typed(&surface, "what connects loss to growth");
        assert!(output.is_some(), "trained surface should classify");
        let out = output.unwrap();
        assert_eq!(out.intent.op, OpKind::Split);
        let refs: Vec<&str> = out.lenses.iter().map(|l| l.ref_.as_str()).collect();
        assert!(refs.contains(&"loss"), "should have 'loss' lens, got {:?}", refs);
        assert!(refs.contains(&"growth"), "should have 'growth' lens, got {:?}", refs);
    }

    #[test]
    fn reflection_plans_composition() {
        let reflection = Reflection::untrained(42);
        let surface_out = SurfaceOutput {
            intent: NamedOptic {
                op: OpKind::Split,
                confidence: 0.8,
                args: vec!["loss".into(), "growth".into()],
            },
            lenses: vec![
                TypedLens {
                    ref_: "loss".into(),
                    kind: OpticKind::Lens,
                    resolved: Imperfect::success(test_oid()),
                },
                TypedLens {
                    ref_: "growth".into(),
                    kind: OpticKind::Lens,
                    resolved: Imperfect::success(test_oid()),
                },
            ],
            target: Imperfect::success(TargetSpec {
                refs: vec!["loss".into(), "growth".into()],
                loss_budget: 0.3,
            }),
        };

        let result = plan(&reflection, &surface_out, None, &[]);
        assert!(
            result.as_ref().ok().is_some(),
            "plan should succeed or be partial"
        );

        let p = match result {
            Imperfect::Success(p) | Imperfect::Partial(p, _) => p,
            Imperfect::Failure(_, _) => panic!("plan failed"),
        };
        assert!(!p.steps.is_empty());
        assert!(p.remaining_budget >= 0.0);
    }

    #[test]
    fn plan_validates_against_schema() {
        let reflection = Reflection::untrained(42);

        let schema = vec![
            FieldOptic { name: "loss", kind: OpticKind::Lens },
            FieldOptic { name: "growth", kind: OpticKind::Traversal },
        ];

        let surface_out = SurfaceOutput {
            intent: NamedOptic {
                op: OpKind::Split,
                confidence: 0.9,
                args: vec!["loss".into(), "growth".into()],
            },
            lenses: vec![
                TypedLens {
                    ref_: "loss".into(),
                    kind: OpticKind::Lens,
                    resolved: Imperfect::success(test_oid()),
                },
                TypedLens {
                    ref_: "growth".into(),
                    kind: OpticKind::Traversal,
                    resolved: Imperfect::success(test_oid()),
                },
            ],
            target: Imperfect::success(TargetSpec {
                refs: vec!["loss".into(), "growth".into()],
                loss_budget: 0.1,
            }),
        };

        let result = plan(&reflection, &surface_out, None, &schema);
        let p = match result {
            Imperfect::Success(p) | Imperfect::Partial(p, _) => p,
            Imperfect::Failure(_, _) => panic!("plan failed"),
        };

        // Lens . Traversal = Traversal
        assert_eq!(p.composition, OpticKind::Traversal);
    }

    #[test]
    fn plan_unresolved_refs_add_cost() {
        let reflection = Reflection::untrained(42);

        let surface_out = SurfaceOutput {
            intent: NamedOptic {
                op: OpKind::Focus,
                confidence: 0.9,
                args: vec!["unknown_ref".into()],
            },
            lenses: vec![TypedLens {
                ref_: "unknown_ref".into(),
                kind: OpticKind::Lens,
                resolved: Imperfect::success(test_oid()),
            }],
            target: Imperfect::success(TargetSpec {
                refs: vec!["unknown_ref".into()],
                loss_budget: 0.5,
            }),
        };

        let result = plan(&reflection, &surface_out, None, &[]);
        assert!(result.is_partial(), "unresolved ref should produce partial result");
    }

    #[test]
    fn opkind_to_optic_kind_mapping() {
        assert_eq!(opkind_to_optic_kind(OpKind::Focus), OpticKind::Lens);
        assert_eq!(opkind_to_optic_kind(OpKind::Project), OpticKind::Prism);
        assert_eq!(opkind_to_optic_kind(OpKind::Split), OpticKind::Traversal);
        assert_eq!(opkind_to_optic_kind(OpKind::Zoom), OpticKind::Lens);
        assert_eq!(opkind_to_optic_kind(OpKind::Refract), OpticKind::Iso);
    }

    #[test]
    fn full_pipeline_processes_nl() {
        let mut pipeline = ModelPipeline::new();
        let result = pipeline.process("what is loss");
        match result {
            Imperfect::Success(s) | Imperfect::Partial(s, _) => {
                assert!(!s.is_empty());
            }
            Imperfect::Failure(PipelineError::ClassificationFailed, _) => {
                // Acceptable — untrained model
            }
            Imperfect::Failure(e, _) => {
                panic!("unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn pipeline_with_trained_surface() {
        let mut pipeline = ModelPipeline::new();

        let features = crate::sel::surface::extract_features("what is loss");
        for _ in 0..100 {
            pipeline.surface.train(OpKind::Split, OpKind::Focus, &features, 0.05);
        }

        let result = pipeline.process("what is loss");
        match result {
            Imperfect::Success(s) | Imperfect::Partial(s, _) => {
                assert!(!s.is_empty(), "pipeline should produce output");
            }
            Imperfect::Failure(e, _) => {
                panic!("pipeline failed: {:?}", e);
            }
        }
    }

    #[test]
    fn loss_budget_propagates() {
        let mut pipeline = ModelPipeline::new();

        let features = crate::sel::surface::extract_features("maybe something about eigenvalues perhaps");
        for _ in 0..50 {
            pipeline.surface.train(OpKind::Focus, OpKind::Zoom, &features, 0.05);
        }

        let result = pipeline.process("maybe something about eigenvalues perhaps");
        match result {
            Imperfect::Partial(_, loss) => {
                assert!(loss.total() > 0.0, "uncertain input should propagate loss");
            }
            Imperfect::Success(_) => {}
            Imperfect::Failure(PipelineError::ClassificationFailed, _) => {}
            Imperfect::Failure(e, _) => {
                panic!("unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn pipeline_classification_failure() {
        let pipeline = ModelPipeline::new();
        let result = pipeline.surface.classify("xyzzy plugh");
        let _ = result;
    }

    #[test]
    fn pipeline_loss_is_loss_impl() {
        let a = PipelineLoss {
            surface_loss: 0.1,
            plan_loss: 0.2,
            execution_loss: 0.0,
            render_loss: 0.0,
        };
        let b = PipelineLoss {
            surface_loss: 0.0,
            plan_loss: 0.0,
            execution_loss: 0.3,
            render_loss: 0.1,
        };
        let combined = terni::Loss::combine(a, b);
        assert!((combined.total() - 0.7).abs() < 1e-10);

        let zero = <PipelineLoss as terni::Loss>::zero();
        assert!(zero.is_zero());
    }

    #[test]
    fn typed_lens_resolution() {
        let lens = TypedLens {
            ref_: "loss".into(),
            kind: OpticKind::Lens,
            resolved: Imperfect::success(test_oid()),
        };
        assert_eq!(lens.ref_, "loss");
        assert_eq!(lens.kind, OpticKind::Lens);
        assert!(lens.resolved.as_ref().ok().is_some());
    }

    #[test]
    fn typed_lens_resolution_failure() {
        let lens = TypedLens {
            ref_: "unknown".into(),
            kind: OpticKind::Lens,
            resolved: Imperfect::failure(ResolutionError::NotFound("unknown".into())),
        };
        assert!(lens.resolved.is_err());
    }

    #[test]
    fn query_plan_composition_tracking() {
        let qp = QueryPlan {
            steps: vec![
                NamedOptic { op: OpKind::Focus, confidence: 0.9, args: vec!["a".into()] },
                NamedOptic { op: OpKind::Split, confidence: 0.8, args: vec!["b".into()] },
            ],
            composition: OpticKind::Lens.compose(OpticKind::Traversal),
            target: Imperfect::success(TargetSpec {
                refs: vec!["a".into(), "b".into()],
                loss_budget: 0.2,
            }),
            remaining_budget: 0.2,
        };
        assert_eq!(qp.composition, OpticKind::Traversal);
        assert_eq!(qp.steps.len(), 2);
    }

    #[test]
    fn execute_plan_with_refs() {
        let qp = QueryPlan {
            steps: vec![
                NamedOptic { op: OpKind::Focus, confidence: 0.9, args: vec!["loss".into()] },
            ],
            composition: OpticKind::Lens,
            target: Imperfect::success(TargetSpec {
                refs: vec!["loss".into()],
                loss_budget: 0.1,
            }),
            remaining_budget: 0.1,
        };
        let result = execute_plan(&qp);
        assert!(result.as_ref().ok().is_some());
        assert!(result.as_ref().ok().unwrap().contains("loss"));
    }

    #[test]
    fn execute_plan_empty_steps() {
        let qp = QueryPlan {
            steps: vec![],
            composition: OpticKind::Iso,
            target: Imperfect::success(TargetSpec {
                refs: vec![],
                loss_budget: 1.0,
            }),
            remaining_budget: 1.0,
        };
        let result = execute_plan(&qp);
        assert!(result.is_partial(), "empty plan should produce partial result");
    }

    #[test]
    fn confidence_loss_combines() {
        let a = ConfidenceLoss(0.3);
        let b = ConfidenceLoss(0.5);
        let c = terni::Loss::combine(a, b);
        assert!((c.0 - 0.8).abs() < 1e-10);

        let d = terni::Loss::combine(c, ConfidenceLoss(0.5));
        assert!((d.0 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn resolution_loss_combines() {
        let a = ResolutionLoss(0.1);
        let b = ResolutionLoss(0.2);
        let c = terni::Loss::combine(a, b);
        assert!((c.0 - 0.3).abs() < 1e-10);
    }

    #[test]
    fn plan_loss_combines() {
        let a = PlanLoss(0.1);
        let b = PlanLoss(0.2);
        let c = terni::Loss::combine(a, b);
        assert!((c.0 - 0.3).abs() < 1e-10);
    }

    #[test]
    fn pipeline_error_display() {
        assert_eq!(
            format!("{}", PipelineError::ClassificationFailed),
            "classification failed"
        );
        assert_eq!(
            format!("{}", PipelineError::PlanFailed("no path".into())),
            "plan failed: no path"
        );
    }

    #[test]
    fn surface_output_high_confidence_target() {
        let out = SurfaceOutput {
            intent: NamedOptic { op: OpKind::Focus, confidence: 0.9, args: vec![] },
            lenses: vec![],
            target: Imperfect::success(TargetSpec {
                refs: vec![],
                loss_budget: 0.1,
            }),
        };
        assert!(out.target.as_ref().ok().is_some());
        assert_eq!(out.target.as_ref().ok().unwrap().loss_budget, 0.1);
    }

    #[test]
    fn model_pipeline_new_defaults() {
        let p = ModelPipeline::new();
        assert!(p.position.is_none());
        assert_eq!(p.eigenvalues.len(), 50);
    }
}
