use crate::aiplan4rust::grounding::binding::evaluator::ExprEvaluator;
use crate::aiplan4rust::grounding::binding::BindingScratchpad;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::passes::qnf::ExpansionScratchpad;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lir::expr::ExprStore;
use crate::aiplan4rust::lir::problem::ActionDef;

use super::expr;

/// Expands all logical quantifiers (`forall` and `exists`) within an action's expressions.
///
/// This is a convenience wrapper around [`expand_with`] that allocates
/// temporary scratchpads locally on the fly.
pub fn expand(
    action: &mut ActionDef,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
) -> Result<(), GroundingError> {
    let mut binding_scratchpad = BindingScratchpad::new();
    let mut expansion_scratchpad = ExpansionScratchpad::new();
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
/// Ce point d'entrée accepte les scratchpads de binding et d'expansion pré-alloués
/// pour garantir l'absence totale d'allocation à chaud lors du traitement des actions.
pub fn expand_with(
    action: &mut ActionDef,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
    binding_scratchpad: &mut BindingScratchpad,
    expansion_scratchpad: &mut ExpansionScratchpad,
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
    // Met à jour la racine de l'expression avec le nouvel ID calculé
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
