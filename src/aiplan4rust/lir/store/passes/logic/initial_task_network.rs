use crate::aiplan4rust::lir::store::expr_old::ops::ExprOpError;
use crate::aiplan4rust::lir::store::passes::logic::task_network;
use crate::aiplan4rust::lir::InitialTaskNetwork;

/// Normalizes an `InitialTaskNetwork` using the provided `LogicEngine`.
///
/// This function applies expression logic only to the `logical_constraints`
/// of the underlying `task_network`.
///
/// # Arguments
///
/// * `binding` - The ops binding to use for logic.
/// * `init` - A mutable reference to the `InitialTaskNetwork` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if logic of the `logical_constraints` fails.
pub fn normalize(init: &mut InitialTaskNetwork) -> Result<(), ExprOpError> {
    task_network::normalize(&mut init.task_network_mut())
}
