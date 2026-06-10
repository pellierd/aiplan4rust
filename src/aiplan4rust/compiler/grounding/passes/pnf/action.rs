use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::pnf::scratchpad::PnfScratchpad;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::problem::ActionDef;
use crate::aiplan4rust::support::lang::AtomSkeletonId;

use super::expr;

/// Applies the Positive Normal Form (PNF) transformation to an action.
///
/// Convenience wrapper around [`to_pnf_with_scratchpad`] that allocates the temporary
/// scratchpad locally on the fly.
pub fn to_pnf(
    action: &mut ActionDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
) -> Result<(), GroundingError> {
    let mut scratchpad = PnfScratchpad::new();
    to_pnf_with_scratchpad(action, store, negated_atoms, &mut scratchpad)
}

/// Applies the Positive Normal Form (PNF) transformation to an action.
///
/// Accepts a pre-allocated `PfnScratchpad` to ensure zero allocations on the hot path
/// while processing massive batches of actions.
pub fn to_pnf_with_scratchpad(
    action: &mut ActionDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut PnfScratchpad,
) -> Result<(), GroundingError> {
    // 1. Collecte et transformation dans les préconditions
    // On passe `is_effect = false` (donc in_condition = true à l'amorçage)
    let old_precondition_id = action.precondition();
    let new_precondition_id = expr::to_pnf_with_scratchpad(
        old_precondition_id,
        store,
        negated_atoms,
        scratchpad,
        false, // is_effect
    )?;
    action.set_precondition(new_precondition_id);

    // 2. Collecte et transformation dans les effets
    // On passe `is_effect = true` (donc in_condition = false à l'amorçage)
    // Note : Le scratchpad est nettoyé automatiquement au début de `expr::to_pnf_with_scratchpad`,
    // mais le vecteur global `negated_atoms` accumule continuellement.
    let old_effect_id = action.effect();
    let new_effect_id = expr::to_pnf_with_scratchpad(
        old_effect_id,
        store,
        negated_atoms,
        scratchpad,
        true, // is_effect
    )?;
    action.set_effect(new_effect_id);

    // 3. Transformation de la durée si elle existe
    if let Some(duration_id) = action.duration() {
        let new_duration_id = expr::to_pnf_with_scratchpad(
            duration_id,
            store,
            negated_atoms,
            scratchpad,
            false, // Une durée se comporte généralement comme une précondition (hors effet)
        )?;
        action.set_duration(new_duration_id);
    }

    Ok(())
}
