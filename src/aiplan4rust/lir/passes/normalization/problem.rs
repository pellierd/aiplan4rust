use crate::aiplan4rust::lir::expr::ops;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::passes::normalization::{action, derived_predicate, initial_task_network, method};

/// Normalizes all normalizable components of a `Problem` using the provided `LogicEngine`.
///
/// This function applies expr to:
/// - Problem-level expressions (`goal`, `domain_constraints`, `problem_constraints`, `metric_spec`)
/// - All derived predicates
/// - All actions
/// - All durative actions
/// - All methods
/// - Initial task network
pub fn normalize(problem: &mut LiftedProblem) -> Result<(), ExprOpError> {
    // Problem-level expr
    ops::normalize(&mut problem.goal_mut())?;
    ops::normalize(&mut problem.domain_constraints_mut())?;
    ops::normalize(&mut problem.problem_constraints_mut())?;
    ops::normalize(&mut problem.metric_spec_mut())?;

    // Normalize all derived predicates
    for derived_predicate in problem.derived_predicate_defs_mut() {
        derived_predicate::normalize(derived_predicate)?;
    }

    // Normalize all actions
    for action in problem.action_defs_mut() {
        action::normalize(action)?;
    }

    // Normalize all methods
    for method in problem.method_def_mut() {
        method::normalize(method)?;
    }

    // Normalize the initial task network
    initial_task_network::normalize(&mut problem.initial_task_network_mut())?;

    Ok(())
}
