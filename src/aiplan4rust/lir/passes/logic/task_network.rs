use crate::aiplan4rust::lir::expr::ops;
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::problem::task_network::TaskNetwork;

/// Normalizes a `TaskNetwork` using the provided `LogicEngine`.
///
/// This function applies expression logic only to the `logical_constraints`
/// field, because `tasks` and `ordering_constraints` are already in canonical form.
///
/// # Arguments
///
/// * `binding` - Reference to the `LogicEngine` to use for logic.
/// * `network` - Mutable reference to the `TaskNetwork` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if logic of the `logical_constraints` fails.
pub fn normalize(network: &mut TaskNetwork) -> Result<(), ExprOpError> {
    ops::normalize(network.logical_constraints_mut())?;
    Ok(())
}
