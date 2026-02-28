use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::passes::quantifier_expansion::expr;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lir::expr::ops::StaticEvaluator;
use crate::aiplan4rust::lir::MethodDef;

/// Expands all logical quantifiers (`forall` and `exists`) within a Method's expressions.
///
/// This is a convenience wrapper around [`expand_with`] without a static evaluator.
pub fn expand(
    method: &mut MethodDef,
    value_registry: &ValueRegistry,
) -> Result<(), GroundingError> {
    expand_with(method, value_registry, None)
}

/// Expands logical quantifiers within a Method, with optional on-the-fly simplification.
///
/// This function processes the hierarchical components of an HTN method:
/// 1. **Preconditions**: Logical requirements for the method to be applicable.
/// 2. **Task Network**: Subtasks and constraints (ordering, timing, and logical constraints).
pub fn expand_with(
    method: &mut MethodDef,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<(), GroundingError> {

    // 1. Expand Preconditions
    // The core of the method's applicability logic.
    let precondition = method.precondition_mut();
    expr::expand_with(
        precondition,
        value_registry,
        evaluator
    )?;

    // 2. Expand Task Network Constraints
    // HTN Task Networks often contain constraints (ordered, at start, etc.)
    // that are stored as expressions.
    let constraints= method.task_network_mut().logical_constraints_mut();
    expr::expand_with(
        constraints,
        value_registry,
        evaluator
    )?;

    Ok(())
}
