use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluator;
use crate::aiplan4rust::compiler::grounding::binding::BindingScratchpad;
use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::qnf::expr;
use crate::aiplan4rust::compiler::grounding::passes::qnf::QnfScratchpad;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::problem::MethodDef;

/// Expands all logical quantifiers (`forall` and `exists`) within a Method's expressions.
///
/// This is a convenience wrapper around [`expand_with`] that allocates
/// temporary scratchpads locally on the fly.
///
/// # Arguments
/// * `method` - A mutable reference to the HTN `MethodDef` whose inner expressions will be grounded.
/// * `store` - A mutable reference to the centralized `ExprStore` used for interning nodes.
/// * `value_registry` - A reference to the `ValueRegistry` holding typing and object instances.
///
/// # Returns
/// * `Ok(())` if both preconditions and task network constraints are successfully expanded.
/// * `Err(GroundingError)` if any structural expansion or evaluation failure occurs.
pub fn expand(
    method: &mut MethodDef,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
) -> Result<(), GroundingError> {
    let mut binding_scratchpad = BindingScratchpad::new();
    let mut expansion_scratchpad = QnfScratchpad::new();
    expand_with(
        method,
        store,
        value_registry,
        None,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )
}

/// Expands logical quantifiers within a Method, with optional on-the-fly simplification.
///
/// This entry point accepts pre-allocated binding and expansion scratchpads to completely
/// avoid dynamic heap allocations during HTN method processing.
///
/// # Arguments
/// * `method` - A mutable reference to the HTN `MethodDef` being processed.
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
    method: &mut MethodDef,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
    binding_scratchpad: &mut BindingScratchpad,
    expansion_scratchpad: &mut QnfScratchpad,
) -> Result<(), GroundingError> {
    // 1. Expand Preconditions
    // Note: If `.precondition()` returns an `Expr` wrapper object, replace with `method.precondition().root_id()`
    let current_precondition_id = method.precondition();
    let new_precondition_id = expr::expand_with(
        current_precondition_id,
        store,
        value_registry,
        evaluator,
        binding_scratchpad,
        expansion_scratchpad,
    )?;
    method.set_precondition(new_precondition_id);

    // 2. Expand Task Network Constraints
    // Note: If `.logical_constraints()` returns an `Expr` wrapper object, replace with `.logical_constraints().root_id()`
    let current_constraints_id = method.task_network().logical_constraints();
    let new_constraints_id = expr::expand_with(
        current_constraints_id,
        store,
        value_registry,
        evaluator,
        binding_scratchpad,
        expansion_scratchpad,
    )?;
    method
        .task_network_mut()
        .set_logical_constraints(new_constraints_id);

    Ok(())
}
