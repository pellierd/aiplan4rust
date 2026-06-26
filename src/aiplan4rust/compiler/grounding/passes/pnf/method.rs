use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::pnf::expr;
use crate::aiplan4rust::compiler::grounding::passes::pnf::scratchpad::PnfScratchpad;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::problem::MethodDef;
use crate::aiplan4rust::support::lang::AtomSkeletonId;

/// Applies the Positive Normal Form (PNF) transformation to an HTN Method definition.
///
/// Convenient entry point wrapper around [`to_pnf_with`] that automatically
/// allocates a temporary, local [`PnfScratchpad`] on the fly.
///
/// # Layout and Optimizations
///
/// While this function is ideal for one-off conversions or isolated test cases,
/// batch-processing pipelines handling massive sets of methods should favor
/// calling [`to_pnf_with`] directly with a single, retained scratchpad
/// to maximize performance and guarantee zero heap allocations.
pub fn to_pnf(
    method: &mut MethodDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
) -> Result<(), GroundingError> {
    // Allocate a localized buffer stack and memoization cache for this single pass
    let mut scratchpad = PnfScratchpad::new();
    to_pnf_with(method, store, negated_atoms, &mut scratchpad)
}

/// Applies the Positive Normal Form (PNF) transformation to an HTN Method definition.
///
/// This method processes both the method's preconditions and the explicit logical
/// constraints declared within its underlying task network, rewriting their syntax trees
/// into their canonical PNF representation.
///
/// # Layout and Optimizations
///
/// Accepts a reusable, pre-allocated [`PnfScratchpad`] to ensure completely zero
/// heap allocations on the hot path while batch-processing large sets of HTN methods.
///
/// * **Context Isolation**: Since HTN method preconditions and task network constraints
///   both evaluate truth states rather than modifying states, `is_effect` is strictly set to `false`.
/// * **SIMD-Driven Cache Reset**: The `scratchpad` internal lookup arenas are zeroed out via fast
///   sequential memory sweeps during the internal `expr::to_pnf_with` clear calls, ensuring zero fragmentation.
/// * **Accumulative Logging**: The global `negated_atoms` vector continuously accumulates
///   every single absorbed atom across the entire HTN method definition.
pub fn to_pnf_with(
    method: &mut MethodDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut PnfScratchpad,
) -> Result<(), GroundingError> {
    // 1. Process and lower the method preconditions
    // We pass `is_effect = false` (initializing context as an evaluation condition)
    let old_precondition_id = method.precondition();
    let new_precondition_id = expr::to_pnf_with(
        old_precondition_id,
        store,
        negated_atoms,
        scratchpad,
        false, // is_effect = false -> context is a query condition
    )?;
    method.set_precondition(new_precondition_id);

    // 2. Process and lower the logical constraints of the inner Task Network
    // Handled directly as a non-optional ExprId node block
    let old_constraints_id = method.task_network().logical_constraints();
    let new_constraints_id = expr::to_pnf_with(
        old_constraints_id,
        store,
        negated_atoms,
        scratchpad,
        false, // is_effect = false -> htn network restrictions are conditions
    )?;
    method
        .task_network_mut()
        .set_logical_constraints(new_constraints_id);

    Ok(())
}
