use crate::aiplan4rust::lir::InitialTaskNetwork;
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::passes::normalization::task_network;

/// Normalizes an `InitialTaskNetwork` using the provided `LogicEngine`.
///
/// This function applies expression expr only to the `logical_constraints`
/// of the underlying `task_network`.
///
/// # Arguments
///
/// * `binding` - The ops binding to use for expr.
/// * `init` - A mutable reference to the `InitialTaskNetwork` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if expr of the `logical_constraints` fails.
pub fn normalize(
    init: &mut InitialTaskNetwork,
) -> Result<(), ExprOpError> {
    task_network::normalize(&mut init.task_network_mut())
}
