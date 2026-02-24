use crate::aiplan4rust::lir::expr::ops;
use crate::aiplan4rust::lir::LiftedMethod;
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::normalization::task_network;

/// Normalizes a `Method` using the provided `LogicEngine`.
///
/// This function applies expression expr to:
/// - `precondition`
/// - `task_network`
///
/// The `task` expression itself is not normalized.
///
/// # Arguments
///
/// * `substitution` - The ops substitution to use for expr.
/// * `method` - A mutable reference to the `Method` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if expr of the precondition or task network fails.
pub fn normalize(
    method: &mut LiftedMethod
) -> Result<(), ExprOpError> {
    ops::normalize(method.precondition_mut())?;
    task_network::normalize(method.task_network_mut())?;
    Ok(())
}
