use crate::aiplan4rust::lir::LiftedDurativeAction;
use crate::aiplan4rust::lir::expr;
use crate::aiplan4rust::lir::expr::ExprError;

/// Normalizes the expressions of a `DurativeAction`.
///
/// This function applies expression normalization to the duration, condition,
/// and effect of the given `DurativeAction`. It is intended for internal use
/// within the module and is not part of the public API.
///
/// Normalization typically rewrites expressions into a canonical form, which
/// simplifies later processing stages such as grounding, validation, or
/// compilation to other representations.
///
/// # Arguments
///
/// * `action` - A mutable reference to the `DurativeAction` to normalize.
///
/// # Errors
///
/// Returns an `ExprError` if normalization of the duration, condition,
/// or effect fails.
///
/// # Example
///
/// ```rust,ignore
/// # use aiplan4rust::lir::DurativeAction;
/// # use aiplan4rust::lir::expr::ExprError;
/// # fn example(action: &mut DurativeAction) -> Result<(), ExprError> {
/// normalize(action)?;
/// # Ok(())
/// # }
/// ```
pub fn normalize(action: &mut LiftedDurativeAction) -> Result<(), ExprError> {
    expr::normalize(action.duration_mut())?;
    expr::normalize(action.condition_mut())?;
    expr::normalize(action.effect_mut())?;
    Ok(())
}
