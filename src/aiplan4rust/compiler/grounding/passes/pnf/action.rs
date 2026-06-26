use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::pnf::expr;
use crate::aiplan4rust::compiler::grounding::passes::pnf::scratchpad::PnfScratchpad;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::problem::ActionDef;
use crate::aiplan4rust::support::lang::AtomSkeletonId;

/// Applies the Positive Normal Form (PNF) transformation to an action definition.
///
/// Convenient entry point wrapper around [`to_pnf_with`] that automatically
/// allocates a temporary, local [`PnfScratchpad`] on the fly.
///
/// # Layout and Optimizations
///
/// While this function is ideal for one-off conversions or isolated test cases,
/// batch-processing pipelines handling massive sets of actions should favor
/// calling [`to_pnf_with`] directly with a single, retained scratchpad
/// to maximize performance and guarantee zero heap allocations.
pub fn to_pnf(
    action: &mut ActionDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
) -> Result<(), GroundingError> {
    // Allocate a localized buffer stack and memoization cache for this single pass
    let mut scratchpad = PnfScratchpad::new();
    to_pnf_with(action, store, negated_atoms, &mut scratchpad)
}

/// Applies the Positive Normal Form (PNF) transformation to an action definition.
///
/// This method processes an action's preconditions, effects, and optional durations,
/// rewriting their underlying expression trees into their canonical PNF representation.
///
/// # Layout and Optimizations
///
/// Accepts a reusable, pre-allocated [`PnfScratchpad`] to ensure completely zero
/// heap allocations on the hot path while batch-processing massive numbers of actions.
///
/// * **Context Isolation**: Automatically toggles the evaluation context (`is_effect`)
///   to ensure logical negations are correctly absorbed within preconditions/durations,
///   while preserving structural negations representing delete effects within effect blocks.
/// * **SIMD-Driven Cache Reset**: The `scratchpad` internal lookup arenas are zeroed out via fast
///   sequential memory sweeps during the internal `expr::to_pnf_with` clear calls, ensuring zero fragmentation.
/// * **Accumulative Logging**: The global `negated_atoms` vector continuously accumulates
///   every single absorbed atom across the entire action definition.
pub fn to_pnf_with(
    action: &mut ActionDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut PnfScratchpad,
) -> Result<(), GroundingError> {
    // 1. Process and lower the action preconditions
    // We pass `is_effect = false` (initializing context as an evaluation condition)
    let old_precondition_id = action.precondition();
    let new_precondition_id = expr::to_pnf_with(
        old_precondition_id,
        store,
        negated_atoms,
        scratchpad,
        false, // is_effect = false -> context is a condition
    )?;
    action.set_precondition(new_precondition_id);

    // 2. Process and lower the action effects
    // We pass `is_effect = true` (initializing context as a structural mutation/delete effect)
    let old_effect_id = action.effect();
    let new_effect_id = expr::to_pnf_with(
        old_effect_id,
        store,
        negated_atoms,
        scratchpad,
        true, // is_effect = true -> context is an effect
    )?;
    action.set_effect(new_effect_id);

    // 3. Process and lower the action duration constraints if they exist
    if let Some(duration_id) = action.duration() {
        let new_duration_id = expr::to_pnf_with(
            duration_id,
            store,
            negated_atoms,
            scratchpad,
            false, // Action durations structurally behave like evaluation conditions
        )?;
        action.set_duration(new_duration_id);
    }

    Ok(())
}
