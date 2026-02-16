use crate::aiplan4rust::lir::{LiftedAction};
use crate::aiplan4rust::lir::logic::{LogicError, LogicEngine};

/// Normalizes an `Action` using the provided `LogicEngine`.
///
/// This function applies expression expr to both the precondition
/// and the effect of the given `Action`. It is intended for internal use
/// within the `problem` module and is not part of the public API.
///
/// # Arguments
///
/// * `engine` - The logic engine to use for expr.
/// * `action` - A mutable reference to the `Action` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if expr of either the precondition or
/// the effect fails.
pub fn normalize(engine: &LogicEngine, action: &mut LiftedAction) -> Result<(), LogicError> {
    // 1. Normalisation de la durée (uniquement si elle existe)
    if let Some(duration_mut) = action.duration_mut() {
        engine.normalize(duration_mut)?;
    }

    // 2. Normalisation de la précondition / condition
    // L'accesseur precondition_mut() renvoie la condition correcte selon le type d'action
    engine.normalize(action.precondition_mut())?;

    // 3. Normalisation de l'effet
    engine.normalize(action.effect_mut())?;

    Ok(())
}
