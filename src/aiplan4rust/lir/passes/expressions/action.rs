use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::{expr, LiftedAction};

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
/// normalize(action)?;
/// # Ok(())
/// # }
/// ```
pub fn normalize(action: &mut LiftedAction) -> Result<(), ExprError> {
    expr::normalize(action.precondition_mut())?;
    expr::normalize(action.effect_mut())?;
    Ok(())
}
