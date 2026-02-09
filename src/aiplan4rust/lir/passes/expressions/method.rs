use crate::aiplan4rust::lir::{expr, LiftedMethod};
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::passes::expressions::task_network;

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
pub fn normalize(method: &mut LiftedMethod) -> Result<(), ExprError> {
    expr::normalize(method.precondition_mut())?;
    task_network::normalize(method.task_network_mut())?;
    Ok(())
}
