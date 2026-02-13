use crate::aiplan4rust::lir::LiftedDurativeAction;
use crate::aiplan4rust::lir::logic::{LogicError, LogicEngine};

/// Normalizes a `DurativeAction` using the provided `LogicEngine`.
///
/// This function applies expression expr to the duration, condition,
/// and effect of the given `DurativeAction`.
///
/// # Arguments
///
/// * `engine` - The logic engine to use for expr.
/// * `action` - A mutable reference to the `DurativeAction` to normalize.
///
/// # Errors
///
/// Returns a `LogicError` if expr of the duration, condition,
/// or effect fails.
pub fn normalize(
    engine: &LogicEngine,
    action: &mut LiftedDurativeAction
) -> Result<(), LogicError> {
    engine.normalize(action.duration_mut())?;
    engine.normalize(action.condition_mut())?;
    engine.normalize(action.effect_mut())?;
    Ok(())
}
