use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::passes::pnf::scratchpad::PnfScratchpad;
use crate::aiplan4rust::lir::expr::ExprStore;
use crate::aiplan4rust::lir::problem::MethodDef;
use crate::aiplan4rust::support::lang::AtomSkeletonId;

use super::expr;

/// Applies the Positive Normal Form (PNF) transformation to an HTN Method.
///
/// Convenience wrapper around [`to_pnf_with_scratchpad`] that allocates the temporary
/// scratchpad locally on the fly.
pub fn to_pnf(
    method: &mut MethodDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
) -> Result<(), GroundingError> {
    let mut scratchpad = PnfScratchpad::new();
    to_pnf_with_scratchpad(method, store, negated_atoms, &mut scratchpad)
}

/// Applies the Positive Normal Form (PNF) transformation to an HTN Method.
///
/// This processes both the method's preconditions and the logical constraints
/// within its task network.
///
/// Accepts a pre-allocated `PfnScratchpad` to ensure zero allocations on the hot path.
pub fn to_pnf_with_scratchpad(
    method: &mut MethodDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut PnfScratchpad,
) -> Result<(), GroundingError> {
    // 1. Transformation des préconditions de la méthode
    let old_precondition_id = method.precondition();
    let new_precondition_id = expr::to_pnf_with_scratchpad(
        old_precondition_id,
        store,
        negated_atoms,
        scratchpad,
        false, // is_effect = false
    )?;
    method.set_precondition(new_precondition_id);

    // 2. Transformation des contraintes logiques du réseau de tâches (Task Network)
    // Pas besoin de `if let Some`, c'est un ExprId direct !
    let old_constraints_id = method.task_network().logical_constraints();
    let new_constraints_id = expr::to_pnf_with_scratchpad(
        old_constraints_id,
        store,
        negated_atoms,
        scratchpad,
        false, // is_effect = false
    )?;
    method
        .task_network_mut()
        .set_logical_constraints(new_constraints_id);

    Ok(())
}
