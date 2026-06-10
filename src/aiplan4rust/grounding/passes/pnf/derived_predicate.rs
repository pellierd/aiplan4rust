use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::passes::pnf::scratchpad::PnfScratchpad;
use crate::aiplan4rust::lir::expr::ExprStore;
use crate::aiplan4rust::lir::problem::DerivedPredicateDef;
use crate::aiplan4rust::support::lang::AtomSkeletonId;

use super::expr;

/// Applies the Positive Normal Form (PNF) transformation to a derived predicate.
///
/// Convenience wrapper around [`to_pnf_with_scratchpad`] that allocates the temporary
/// scratchpad locally on the fly.
pub fn to_pnf(
    predicate: &mut DerivedPredicateDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
) -> Result<(), GroundingError> {
    let mut scratchpad = PnfScratchpad::new();
    to_pnf_with_scratchpad(predicate, store, negated_atoms, &mut scratchpad)
}

/// Applies the Positive Normal Form (PNF) transformation to a derived predicate.
///
/// Derived predicates define a formula (the body) that implies the head.
/// We apply PNF to the entire body to eliminate structural negation.
///
/// This version accepts a pre-allocated `PfnScratchpad` to ensure zero allocations
/// on the hot path while processing batches of predicates.
pub fn to_pnf_with_scratchpad(
    predicate: &mut DerivedPredicateDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut PnfScratchpad,
) -> Result<(), GroundingError> {
    // Le corps (body) d'un prédicat dérivé définit une condition logique.
    // On applique PNF avec `is_effect = false` (donc `in_condition = true` à l'amorçage).
    let old_body_id = predicate.body();

    let new_body_id = expr::to_pnf_with_scratchpad(
        old_body_id,
        store,
        negated_atoms,
        scratchpad,
        false, // is_effect = false car c'est une formule de condition
    )?;

    // Met à jour la racine du corps avec le nouvel ID calculé et interné
    predicate.set_body(new_body_id);

    Ok(())
}
