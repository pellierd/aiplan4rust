use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::passes::quantifier_expansion::{
    action, derived_predicate, expr, method,
};
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lir::store::expr_old::ops::StaticEvaluator;
use crate::aiplan4rust::lir::store::problem_old::LiftedProblem;
use crate::aiplan4rust::lir::store::renderers_old::RenderContext;
use crate::aiplan4rust::lir::ActionDef;

/// Fully expands all logical quantifiers across the entire planning problem.
///
/// This convenience wrapper calls [`expand_with`] without a static evaluator.
pub fn expand(
    problem: &mut LiftedProblem,
    value_registry: &ValueRegistry,
) -> Result<(), GroundingError> {
    expand_with(problem, value_registry, None)
}

/// Fully expands logical quantifiers across the entire problem with optional simplification.
///
/// This function acts as the orchestrator for the grounding pass, processing:
/// 1. **Domain & Problem Constraints**: Global logical invariants.
/// 2. **Definitions**: Actions, Methods, and Derived Predicates.
/// 3. **Problem Specifics**: Init, Goal, Metric, and Initial Task Network.
pub fn expand_with(
    problem: &mut LiftedProblem,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<(), GroundingError> {
    // --- 1. Global Constraints ---
    expr::expand_with(
        &mut problem.domain_constraints_mut(),
        value_registry,
        evaluator,
    )?;
    expr::expand_with(
        &mut problem.problem_constraints_mut(),
        value_registry,
        evaluator,
    )?;

    // --- 2. Lifted Definitions (Action, Methods, Derived Predicates) ---
    for derived in problem.derived_predicate_defs_mut() {
        derived_predicate::expand_with(derived, value_registry, evaluator)?;
    }

    // --- 1. DEBUG AVANT (dans un bloc isolé) ---
    {
        let render_ctx = RenderContext::new(problem);
        for action_def in problem.action_defs() {
            if render_ctx
                .resolve_action_symbol(action_def.name())
                .contains("assemble")
            {
                println!("--- DEBUG ACTION AVANT EXPANSION ---");
                struct ActionDisplay<'a>(&'a ActionDef, &'a RenderContext<'a>);
                impl std::fmt::Display for ActionDisplay<'_> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        crate::aiplan4rust::lir::store::renderers_old::syntax::action::render(
                            f, self.0, self.1,
                        )
                    }
                }
                println!("{}", ActionDisplay(action_def, &render_ctx));
                println!("------------------------------------");
            }
        }
    } // <--- Ici, render_ctx est détruit, l'emprunt immuable s'arrête.

    // --- 2. MODIFICATION MUTABLE ---
    for action_def in problem.action_defs_mut() {
        action::expand_with(action_def, value_registry, evaluator)?;
    }

    // --- 3. DEBUG APRÈS (on recrée un contexte) ---
    {
        let render_ctx = RenderContext::new(problem);
        for action_def in problem.action_defs() {
            if render_ctx
                .resolve_action_symbol(action_def.name())
                .contains("assemble")
            {
                println!("--- DEBUG ACTION APRES EXPANSION ---");
                struct ActionDisplay<'a>(&'a ActionDef, &'a RenderContext<'a>);
                impl std::fmt::Display for ActionDisplay<'_> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        crate::aiplan4rust::lir::store::renderers_old::syntax::action::render(
                            f, self.0, self.1,
                        )
                    }
                }
                println!("{}", ActionDisplay(action_def, &render_ctx));
                println!("------------------------------------");
            }
        }
    }

    for method_def in problem.method_defs_mut() {
        method::expand_with(method_def, value_registry, evaluator)?;
    }

    // --- 3. Problem Instance Specifics ---

    // Goal is a primary target for quantifiers (e.g., "all trucks are at base").
    expr::expand_with(&mut problem.goal_mut(), value_registry, evaluator)?;

    // Optimization metrics.
    expr::expand_with(&mut problem.metric_spec_mut(), value_registry, evaluator)?;

    // --- 4. HTN Initial Task Network ---
    let constraints = problem
        .initial_task_network_mut()
        .task_network_mut()
        .logical_constraints_mut();
    expr::expand_with(constraints, value_registry, evaluator)?;

    Ok(())
}
