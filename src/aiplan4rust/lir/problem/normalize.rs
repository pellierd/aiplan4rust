use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::problem::action::Action;
use crate::aiplan4rust::lir::expr;
use crate::aiplan4rust::lir::problem::InitialTaskNetwork;
use crate::aiplan4rust::lir::problem::method::Method;
use crate::aiplan4rust::lir::problem::task_network::TaskNetwork;

/// Normalizes the expressions of an `Action`.
///
/// This function applies expression normalization to both the precondition
/// and the effect of the given `Action`. It is intended for internal use
/// within the `problem` module and is not part of the public API.
///
/// # Arguments
///
/// * `action` - A mutable reference to the `Action` to normalize.
///
/// # Errors
///
/// Returns an `ExprError` if normalization of either the precondition or
/// the effect fails.
///
/// # Example
///
/// ```rust,ignore
/// # use aiplan4rust::lir::Action;
/// # use aiplan4rust::lir::expr::ExprError;
/// # fn example(action: &mut Action) -> Result<(), ExprError> {
/// normalize_action(action)?;
/// # Ok(())
/// # }
/// ```
pub(crate) fn normalize_action(action: &mut Action) -> Result<(), ExprError> {
    expr::normalize(action.precondition_mut())?;
    expr::normalize(action.effect_mut())?;
    Ok(())
}

/// Normalizes the expressions of a `Method`.
///
/// This function applies expression normalization to the parts of a `Method`
/// that can contain arbitrary logical expressions:
/// - `precondition`: the applicability condition of the method.
/// - `task_network`: the subtasks and their logical constraints.
///
/// The `task` expression of the method is not normalized because it is
/// always a simple atomic expression representing the task being decomposed,
/// so no normalization is necessary.
///
/// This function is intended for internal use within the `problem` module
/// and is not part of the public API.
///
/// # Arguments
///
/// * `method` - A mutable reference to the `Method` to normalize.
///
/// # Errors
///
/// Returns an `ExprError` if normalization of the precondition or any
/// logical constraints in the task network fails.
pub(crate) fn normalize_method(method: &mut Method) -> Result<(), ExprError> {
    expr::normalize(method.precondition_mut())?;
    normalize_task_network(method.task_network_mut())?;
    Ok(())
}

/// Normalizes the expressions of a `TaskNetwork`.
///
/// This function applies expression normalization only to the `logical_constraints`
/// field, because `tasks` and `ordering_constraints` are always in a fixed canonical form
/// (e.g., `tasks` is a simple conjunction of subtasks, and `ordering_constraints`
/// is a conjunction of ordering expressions), so normalization is unnecessary for them.
///
/// This function is intended for internal use within the `problem` module
/// and is not part of the public API.
///
/// # Arguments
///
/// * `network` - A mutable reference to the `TaskNetwork` to normalize.
///
/// # Errors
///
/// Returns an `ExprError` if normalization of the `logical_constraints` fails.
pub(crate) fn normalize_task_network(network: &mut TaskNetwork) -> Result<(), ExprError> {
    expr::normalize(network.logical_constraints_mut())?;
    Ok(())
}

/// Normalizes the expressions of an `InitialTaskNetwork`.
///
/// This function applies expression normalization only to the `logical_constraints`
/// of the underlying `task_network`. The `tasks` and `ordering_constraints`
/// fields are always in a fixed canonical form and do not require normalization.
///
/// This function is intended for internal use within the `problem` module
/// and is not part of the public API.
///
/// # Arguments
///
/// * `init` - A mutable reference to the `InitialTaskNetwork` to normalize.
///
/// # Errors
///
/// Returns an `ExprError` if normalization of the `logical_constraints` fails.
pub(crate) fn normalize_initial_task_network(
    init: &mut InitialTaskNetwork,
) -> Result<(), ExprError> {
    normalize_task_network(&mut init.task_network_mut())
}
