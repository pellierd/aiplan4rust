use crate::aiplan4rust::compiler::lir::expr::iter::Scratchpad;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::normalization::error::NormalizationError;
use crate::aiplan4rust::compiler::lir::normalization::logic::expr;
use crate::aiplan4rust::compiler::lir::problem::ActionDef;

/// Normalizes an `Action` using the canonical expression normalization pipeline.
///
/// This function applies simplification and rewriting passes to the precondition,
/// the effect, and (if applicable) the duration constraints of the given `Action`
/// via `expr::normalize`.
///
/// # Arguments
///
/// * `action` - A mutable reference to the `ActionDef` to normalize.
/// * `old` - The central `ExprStore` containing the expression nodes.
/// * `scratch` - A reusable `Scratchpad` to avoid heap allocations during rewrite passes.
///
/// # Errors
///
/// Returns an `ExprOpErrorHC` if the normalization or simplification of any
/// inner expression fails.
pub fn normalize(
    action: &mut ActionDef,
    store: &mut ExprStore,
    scratch: &mut Scratchpad,
) -> Result<(), NormalizationError> {
    let is_durative = action.is_durative();

    // 1. Duration (only applicable for durative actions)
    if is_durative {
        if let Some(duration) = action.duration() {
            // Inside this block, it is guaranteed to be a durative temporal context.
            let normalized_duration = expr::normalize(duration, store, scratch, true)?;
            action.set_duration(normalized_duration);
        }
    }

    // 2. Preconditions and Effects (common to all action types)
    let normalized_precondition =
        expr::normalize(action.precondition(), store, scratch, is_durative)?;
    action.set_precondition(normalized_precondition);
    let normalized_effect = expr::normalize(action.effect(), store, scratch, is_durative)?;
    action.set_effect(normalized_effect);

    Ok(())
}
