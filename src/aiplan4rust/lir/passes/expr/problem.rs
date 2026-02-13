use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::logic::{LogicError, LogicEngine};
use crate::aiplan4rust::lir::passes::expr::{
    action, derived_predicate, durative_action, initial_task_network, method,
};

/// Normalizes all normalizable components of a `Problem` using the provided `LogicEngine`.
///
/// This function applies expr to:
/// - Problem-level expressions (`goal`, `domain_constraints`, `problem_constraints`, `metric_spec`)
/// - All derived predicates
/// - All actions
/// - All durative actions
/// - All methods
/// - Initial task network
pub fn normalize(problem: &mut LiftedProblem) -> Result<(), LogicError> {
    let engine = LogicEngine::new();
    // Problem-level expr
    engine.normalize(&mut problem.goal_mut())?;
    engine.normalize(&mut problem.domain_constraints_mut())?;
    engine.normalize(&mut problem.problem_constraints_mut())?;
    engine.normalize(&mut problem.metric_spec_mut())?;

    // Normalize all derived predicates
    for derived_predicate in problem.derived_predicate_defs_mut() {
        derived_predicate::normalize(&engine, derived_predicate)?;
    }

    // Normalize all actions
    for action in problem.action_defs_mut() {
        action::normalize(&engine, action)?;
    }

    // Normalize all durative actions
    for action in problem.durative_action_def_mut() {
        durative_action::normalize(&engine, action)?;
    }

    // Normalize all methods
    for method in problem.method_def_mut() {
        method::normalize(&engine, method)?;
    }

    // Normalize the initial task network
    initial_task_network::normalize(&engine, &mut problem.initial_task_network_mut())?;

    Ok(())
}
