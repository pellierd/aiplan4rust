use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluator;
use crate::aiplan4rust::compiler::grounding::binding::BindingScratchpad;
use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::qnf::{
    action, derived_predicate, expr, method, QnfScratchpad,
};
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;

/// Fully expands all logical quantifiers across the entire planning problem.
///
/// This convenience wrapper calls [`expand_with`] without a static evaluator.
///
/// # Arguments
/// * `problem` - A mutable reference to the `LiftedProblem` to be expanded.
/// * `value_registry` - A reference to the `ValueRegistry` containing domain objects.
///
/// # Returns
/// * `Ok(())` on success.
/// * `Err(GroundingError)` if any part of the problem expansion fails.
pub fn expand(
    problem: &mut LiftedProblem,
    value_registry: &ValueRegistry,
) -> Result<(), GroundingError> {
    expand_with(problem, value_registry, None)
}

/// Fully expands logical quantifiers across the entire problem with optional simplification.
///
/// # Arguments
/// * `problem` - A mutable reference to the `LiftedProblem` to be processed.
/// * `value_registry` - A reference to the `ValueRegistry` listing available domain objects per type.
/// * `evaluator` - An optional reference to an `ExprEvaluator` implementation for partial compile-time evaluation.
///
/// # Returns
/// * `Ok(())` if the problem constraints, definitions, and goals were expanded successfully.
/// * `Err(GroundingError)` if an error is encountered during the grounding of any expression or definition.
pub fn expand_with(
    problem: &mut LiftedProblem,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
) -> Result<(), GroundingError> {
    // --- SINGLE ALLOCATION OF SCRATCHPADS FOR THE ENTIRE PROBLEM ---
    let mut binding_scratchpad = BindingScratchpad::new();
    let mut expansion_scratchpad = QnfScratchpad::new();

    // 1. Fully extract ownership of the expression store.
    let mut store = problem.take_store();

    // --- 2. Global Constraints ---
    let new_domain_constraints = expr::expand_with(
        problem.domain_constraints(),
        &mut store,
        value_registry,
        evaluator,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )?;
    problem.set_domain_constraints(new_domain_constraints);

    let new_problem_constraints = expr::expand_with(
        problem.problem_constraints(),
        &mut store,
        value_registry,
        evaluator,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )?;
    problem.set_problem_constraints(new_problem_constraints);

    // --- 3. Lifted Definitions (Derived Predicates) ---
    for derived in problem.derived_predicate_defs_mut() {
        derived_predicate::expand_with(
            derived,
            &mut store,
            value_registry,
            evaluator,
            &mut binding_scratchpad,
            &mut expansion_scratchpad,
        )?;
    }

    // --- 4. Lifted Definitions (Actions) ---
    for action_def in problem.action_defs_mut() {
        action::expand_with(
            action_def,
            &mut store,
            value_registry,
            evaluator,
            &mut binding_scratchpad,
            &mut expansion_scratchpad,
        )?;
    }

    // --- 5. Lifted Definitions (Methods HTN) ---
    for method_def in problem.method_defs_mut() {
        method::expand_with(
            method_def,
            &mut store,
            value_registry,
            evaluator,
            &mut binding_scratchpad,
            &mut expansion_scratchpad,
        )?;
    }

    // --- 6. Problem Instance Specifics (Goal & Metrics) ---
    let new_goal = expr::expand_with(
        problem.goal(),
        &mut store,
        value_registry,
        evaluator,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )?;
    problem.set_goal(new_goal);

    let new_metric = expr::expand_with(
        problem.metric_spec(),
        &mut store,
        value_registry,
        evaluator,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )?;
    problem.set_metric_spec(new_metric);

    // --- 7. HTN Initial Task Network ---
    let current_htn_constraints = problem
        .initial_task_network()
        .task_network()
        .logical_constraints();

    let new_htn_constraints = expr::expand_with(
        current_htn_constraints,
        &mut store,
        value_registry,
        evaluator,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )?;
    problem
        .initial_task_network_mut()
        .task_network_mut()
        .set_logical_constraints(new_htn_constraints);

    // 8. Re-inject the updated expression store back into the problem (Restore)
    problem.set_store(store);

    Ok(())
}
