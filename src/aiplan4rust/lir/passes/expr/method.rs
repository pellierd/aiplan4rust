use crate::aiplan4rust::lir::{LiftedMethod};
use crate::aiplan4rust::lir::logic::{LogicError, LogicEngine};
use crate::aiplan4rust::lir::passes::expr::task_network;

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
/// * `engine` - The logic engine to use for expr.
/// * `method` - A mutable reference to the `Method` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if expr of the precondition or task network fails.
pub fn normalize(
    engine: &LogicEngine,
    method: &mut LiftedMethod
) -> Result<(), LogicError> {
    engine.normalize(method.precondition_mut())?;
    task_network::normalize(engine, method.task_network_mut())?;
    Ok(())
}
