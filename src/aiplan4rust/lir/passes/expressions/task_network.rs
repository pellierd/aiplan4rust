use crate::aiplan4rust::lir::expr;
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::task_network::TaskNetwork;

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
pub fn normalize(network: &mut TaskNetwork) -> Result<(), ExprError> {
    expr::normalize(network.logical_constraints_mut())?;
    Ok(())
}
