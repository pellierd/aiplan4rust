use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::pnf::expr;
use crate::aiplan4rust::compiler::grounding::passes::pnf::scratchpad::PnfScratchpad;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::problem::DerivedPredicateDef;
use crate::aiplan4rust::support::lang::AtomSkeletonId;

/// Applies the Prenex Normal Form (PNF) transformation to a derived predicate definition.
///
/// Convenient entry point wrapper around [`to_pnf_with_scratchpad`] that automatically
/// allocates a temporary, local [`PnfScratchpad`] on the fly.
///
/// # Layout and Optimizations
///
/// While this function is ideal for one-off conversions or isolated test cases,
/// batch-processing pipelines handling massive sets of derived predicates should favor
/// calling [`to_pnf_with_scratchpad`] directly with a single, retained scratchpad
/// to maximize performance and guarantee zero heap allocations.
pub fn to_pnf(
    predicate: &mut DerivedPredicateDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
) -> Result<(), GroundingError> {
    // Allocate a localized buffer stack and memoization cache for this single pass
    let mut scratchpad = PnfScratchpad::new();
    to_pnf_with_scratchpad(predicate, store, negated_atoms, &mut scratchpad)
}

/// Applies the Prenex Normal Form (PNF) transformation to a derived predicate definition.
///
/// Derived predicates define a logical formula (the body) that implies the predicate head.
/// This method processes the entire logical body to eliminate structural negation and rewrite
/// its underlying tree into its canonical PNF representation.
///
/// # Layout and Optimizations
///
/// Accepts a reusable, pre-allocated [`PnfScratchpad`] to ensure completely zero
/// heap allocations on the hot path while batch-processing large sets of derived predicates.
///
/// * **Context Isolation**: Since a derived predicate body defines a pure logical condition
///   rather than an execution effect, `is_effect` is strictly set to `false`.
/// * **Accumulative Logging**: The `scratchpad` cache is cleared transparently between
///   sub-tree passes, while the global `negated_atoms` vector continuously accumulates
///   every single absorbed atom across the entire derived predicate definition.
pub fn to_pnf_with_scratchpad(
    predicate: &mut DerivedPredicateDef,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut PnfScratchpad,
) -> Result<(), GroundingError> {
    // The body of a derived predicate evaluates true/false conditions.
    // We execute the pass with `is_effect = false` (initializing context as an evaluation condition).
    let old_body_id = predicate.body();

    let new_body_id = expr::to_pnf_with(
        old_body_id,
        store,
        negated_atoms,
        scratchpad,
        false, // is_effect = false
    )?;

    // Update the derived predicate body with the newly restructured and interned expression ID
    predicate.set_body(new_body_id);

    Ok(())
}
