use crate::aiplan4rust::lir::store::expr_old::ops;
use crate::aiplan4rust::lir::store::expr_old::ops::ExprOpError;
use crate::aiplan4rust::lir::store::passes::logic::task_network;
use crate::aiplan4rust::lir::MethodDef;

/// Normalizes a `Method` using the provided `LogicEngine`.
///
/// This function applies expression logic to:
/// - `precondition`
/// - `task_network`
///
/// The `task` expression itself is not normalized.
///
/// # Arguments
///
/// * `binding` - The ops binding to use for logic.
/// * `method` - A mutable reference to the `Method` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if logic of the precondition or task network fails.
pub fn normalize(method: &mut MethodDef) -> Result<(), ExprOpError> {
    ops::normalize(method.precondition_mut())?;
    task_network::normalize(method.task_network_mut())?;
    Ok(())
}
