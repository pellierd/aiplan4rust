use crate::aiplan4rust::lir::ActionDef;
use crate::aiplan4rust::lir::expr::ops;
use crate::aiplan4rust::lir::expr::ops::ExprOpError;

/// Normalizes an `Action` using the provided `LogicEngine`.
///
/// This function applies expression expr to both the precondition
/// and the effect of the given `Action`. It is intended for internal use
/// within the `problem` module and is not part of the public API.
///
/// # Arguments
///
/// * `action` - A mutable reference to the `Action` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if expr of either the precondition or
/// the effect fails.
pub fn normalize(action: &mut ActionDef) -> Result<(), ExprOpError> {
    // 1. Normalisation de la durée (uniquement si elle existe)
    if let Some(duration_mut) = action.duration_mut() {
        ops::normalize(duration_mut)?;
    }

    // 2. Normalisation de la précondition / condition
    // L'accesseur precondition_mut() renvoie la condition correcte selon le either_type d'action
    ops::normalize(action.precondition_mut())?;

    // 3. Normalisation de l'effet
    ops::normalize(action.effect_mut())?;

    Ok(())
}
