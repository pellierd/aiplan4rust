use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluator;
use crate::aiplan4rust::compiler::grounding::binding::BindingScratchpad;
use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::qnf::expr;
use crate::aiplan4rust::compiler::grounding::passes::qnf::QnfScratchpad;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::problem::ActionDef;

/// Expands all logical quantifiers (`forall` and `exists`) within an action's expressions.
///
/// This is a convenience wrapper around [`expand_with`] that allocates
/// temporary scratchpads locally on the fly.
///
/// # Arguments
/// * `action` - A mutable reference to the `ActionDef` whose inner expressions (preconditions, effects, durations) will be grounded.
/// * `store` - A mutable reference to the centralized `ExprStore` used for interning nodes.
/// * `value_registry` - A reference to the `ValueRegistry` holding typing and object instances.
///
/// # Returns
/// * `Ok(())` if all action expression components are successfully expanded.
/// * `Err(GroundingError)` if any structural expansion or evaluation failure occurs.
pub fn expand(
    action: &mut ActionDef,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
) -> Result<(), GroundingError> {
    let mut binding_scratchpad = BindingScratchpad::new();
    let mut expansion_scratchpad = QnfScratchpad::new();
    expand_with(
        action,
        store,
        value_registry,
        None,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )
}

/// Expands logical quantifiers within an action, with optional on-the-fly simplification.
///
/// This entry point accepts pre-allocated binding and expansion scratchpads to completely
/// avoid dynamic heap allocations during action processing.
///
/// # Arguments
/// * `action` - A mutable reference to the `ActionDef` being processed.
/// * `store` - A mutable reference to the expression store.
/// * `value_registry` - A reference to the available domain objects registry.
/// * `evaluator` - An optional static evaluator for on-the-fly logical simplifications.
/// * `binding_scratchpad` - A reusable arena for variable-to-value assignments.
/// * `expansion_scratchpad` - A reusable arena for non-recursive tree traversals and caching.
///
/// # Returns
/// * `Ok(())` on a successful grounding pass.
/// * `Err(GroundingError)` if expansion fails on any sub-expression tree.
pub fn expand_with(
    action: &mut ActionDef,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
    binding_scratchpad: &mut BindingScratchpad,
    expansion_scratchpad: &mut QnfScratchpad,
) -> Result<(), GroundingError> {
    // 1. Expand Precondition (or Temporal Condition)
    let precondition = action.precondition();
    let new_precondition_id = expr::expand_with(
        precondition,
        store,
        value_registry,
        evaluator,
        binding_scratchpad,
        expansion_scratchpad,
    )?;
    // Updates the root expression with the newly computed ID
    action.set_precondition(new_precondition_id);

    // 2. Expand Effects
    let effect = action.effect();
    let new_effect_id = expr::expand_with(
        effect,
        store,
        value_registry,
        evaluator,
        binding_scratchpad,
        expansion_scratchpad,
    )?;
    action.set_effect(new_effect_id);

    // 3. Expand Duration (Temporal Actions only)
    if let Some(duration) = action.duration() {
        let new_duration_id = expr::expand_with(
            duration,
            store,
            value_registry,
            evaluator,
            binding_scratchpad,
            expansion_scratchpad,
        )?;
        action.set_duration(new_duration_id);
    }

    Ok(())
}
