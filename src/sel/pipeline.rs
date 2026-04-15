//! Model composition pipeline: Surface -> Reflection -> Mirror -> Shatter.
//!
//! Wires the four NL models into a typed pipeline where each stage's output
//! feeds the next. Loss budget propagates through the chain.
//! Optic composition is validated at planning time via OpticKind::compose.

// Types to implement: TypedLens, SurfaceOutput, TargetSpec, QueryPlan, ModelPipeline
// Loss types: PipelineLoss, ResolutionLoss, ConfidenceLoss, PlanLoss
// Functions: classify_typed, plan, execute_plan, process

#[cfg(test)]
mod tests {
    use prism::OpticKind;

    #[test]
    fn pipeline_types_exist() {
        // These types don't exist yet — compile failure expected
        let _: super::TypedLens;
        let _: super::SurfaceOutput;
        let _: super::QueryPlan;
        let _: super::ModelPipeline;
    }

    #[test]
    fn opkind_mapping_exists() {
        let _kind = super::opkind_to_optic_kind(crate::sel::surface::OpKind::Focus);
        assert_eq!(_kind, OpticKind::Lens);
    }
}
