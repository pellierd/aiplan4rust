use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::expr;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::passes::expressions::{action, derived_predicate, durative_action, initial_task_network, method, task_network};

/// Normalizes all expressions and normalizable components of a `Problem`.
///
/// This function applies expression normalization to:
/// - All actions (`precondition` and `effect`)
/// - All methods (`precondition` and logical constraints in their task networks)
/// - The initial task network (`logical_constraints` only)
/// - Problem-level expressions: `domain_constraints`, `init`, `goal`, `problem_constraints`,
///   `metric_spec`, and `length_spec`
///
/// # Arguments
///
/// * `problem` - A mutable reference to the `Problem` to normalize.
///
/// # Errors
///
/// Returns an `ExprError` if normalization fails for any expression.
pub fn normalize(problem: &mut LiftedProblem) -> Result<(), ExprError> {
    // Normalize problem-level expressions
    expr::normalize(&mut problem.goal_mut())?;
    expr::normalize(&mut problem.domain_constraints_mut())?;
    expr::normalize(&mut problem.problem_constraints_mut())?;
    expr::normalize(&mut problem.metric_spec_mut())?;

    // Normalize all derived predicates
    for derived_predicate in problem.derived_predicate_defs_mut() {
        derived_predicate::normalize(derived_predicate)?;
    }

    // Normalize all actions
    for action in problem.action_defs_mut() {
        action::normalize(action)?;
    }

    // Normalize all durative actions
    for action in problem.durative_action_def_mut() {
        durative_action::normalize(action)?;
    }

    // Normalize all methods
    for method in problem.method_def_mut() {
        method::normalize(method)?;
    }

    // Normalize the initial task network
    initial_task_network::normalize(&mut problem.initial_task_network_mut())?;

    Ok(())
}
