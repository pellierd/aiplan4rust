use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::passes::quantifier_expansion::expr;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lir::store::expr_old::ops::StaticEvaluator;
use crate::aiplan4rust::lir::ActionDef;

/// Expands all logical quantifiers (`forall` and `exists`) within an action's expressions.
///
/// This is a convenience wrapper around [`expand_with`] that performs expansion
/// without any additional static simplification.
///
/// # Arguments
/// * `action` - A mutable reference to the lifted action to transform.
/// * `value_registry` - The evaluator containing all object constants used for variable substitution.
///
/// # Errors
/// Returns [`GroundingError`] if a quantifier references an unknown typing or if
/// the expansion exceeds system limits.
pub fn expand(
    action: &mut ActionDef,
    value_registry: &ValueRegistry,
) -> Result<(), GroundingError> {
    expand_with(action, value_registry, None)
}

/// Expands logical quantifiers within an action, with optional on-the-fly simplification.
///
/// This function processes the three main components of a PDDL action:
/// 1. **Preconditions**: Logical requirements (including temporal conditions for durative actions).
/// 2. **Effects**: State changes (including conditional effects).
/// 3. **Duration**: Temporal constraints (only for durative actions).
///
/// If a `StaticEvaluator` is provided, the function will attempt to simplify
/// expressions (e.g., constant folding) immediately after expanding each quantifier.
///
/// # Arguments
/// * `action` - A mutable reference to the lifted action to transform.
/// * `value_registry` - The evaluator containing the domain's object information.
/// * `evaluator` - An optional reference to a static evaluator for immediate simplification.
///
/// # Errors
/// Returns [`GroundingError`] if the expansion fails at any expression level.
pub fn expand_with(
    action: &mut ActionDef,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<(), GroundingError> {
    // 1. Expand Precondition (or Temporal Condition)
    // For durative actions, this targets the entire condition tree.
    let precondition = action.precondition_mut();
    expr::expand_with(precondition, value_registry, evaluator)?;

    // 2. Expand Effects
    // This handles both simple and conditional effects (when-clauses).
    let effect = action.effect_mut();
    expr::expand_with(effect, value_registry, evaluator)?;

    // 3. Expand Duration (Temporal Actions only)
    // Essential if duration constraints involve parameters or numeric fluents.
    if let Some(duration) = action.duration_mut() {
        expr::expand_with(duration, value_registry, evaluator)?;
    }

    Ok(())
}
