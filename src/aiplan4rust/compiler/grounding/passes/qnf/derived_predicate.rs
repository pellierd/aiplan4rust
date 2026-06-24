use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluator;
use crate::aiplan4rust::compiler::grounding::binding::BindingScratchpad;
use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::qnf::expr;
use crate::aiplan4rust::compiler::grounding::passes::qnf::QnfScratchpad;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::problem::derived_predicate::DerivedPredicate;

/// Expands all logical quantifiers (`forall` and `exists`) within the predicate's body.
///
/// This is a convenience wrapper around [`expand_with`] that allocates
/// temporary scratchpads locally on the fly.
///
/// # Arguments
/// * `predicate` - A mutable reference to the `DerivedPredicate` whose inner body expressions will be grounded.
/// * `store` - A mutable reference to the centralized `ExprStore` used for interning nodes.
/// * `value_registry` - A reference to the `ValueRegistry` holding typing and object instances.
///
/// # Returns
/// * `Ok(())` if the predicate body is successfully expanded.
/// * `Err(GroundingError)` if any structural expansion or evaluation failure occurs.
pub fn expand(
    predicate: &mut DerivedPredicate,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
) -> Result<(), GroundingError> {
    let mut binding_scratchpad = BindingScratchpad::new();
    let mut expansion_scratchpad = QnfScratchpad::new();
    expand_with(
        predicate,
        store,
        value_registry,
        None,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )
}

/// Expands logical quantifiers within the predicate's body with optional simplification.
///
/// This entry point accepts a mutable reference to the full derived predicate, extracts its
/// inner `ExprId`, and applies the update directly via its setter.
///
/// # Arguments
/// * `predicate` - A mutable reference to the `DerivedPredicate` being processed.
/// * `store` - A mutable reference to the expression store.
/// * `value_registry` - A reference to the available domain objects registry.
/// * `evaluator` - An optional static evaluator for on-the-fly logical simplifications.
/// * `binding_scratchpad` - A reusable arena for variable-to-value assignments.
/// * `expansion_scratchpad` - A reusable arena for non-recursive tree traversals and caching.
///
/// # Returns
/// * `Ok(())` on a successful grounding pass.
/// * `Err(GroundingError)` if expansion fails on the predicate's body expression tree.
pub fn expand_with(
    predicate: &mut DerivedPredicate,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
    binding_scratchpad: &mut BindingScratchpad,
    expansion_scratchpad: &mut QnfScratchpad,
) -> Result<(), GroundingError> {
    // 1. Extract the current body ID by copy (primitive type, no borrowing conflicts)
    let body = predicate.body();

    // 2. Delegate the expansion task to the core expression-level algorithm
    let new_body_id = expr::expand_with(
        body,
        store,
        value_registry,
        evaluator,
        binding_scratchpad,
        expansion_scratchpad,
    )?;

    // 3. Mutate the derived predicate instance directly with the newly interned ID
    predicate.set_body(new_body_id);

    Ok(())
}
