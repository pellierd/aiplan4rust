//! # Action Definition Flattening
//!
//! This module implements the type resolution and flattening logic for primitive actions.
//!
//! ## Overview
//! Actions are the fundamental operators of a planning domain. Flattening
//! an action requires a consistent transformation of its signature and its
//! logical components to ensure that the grounded state space remains valid and
//! computationally efficient.
//!
//! The transformation process follows a three-step pipeline:
//! 1. **Signature Resolution**: Converting composite parameter types into
//!    unified atomic identifiers.
//! 2. **Precondition Resolution**: Resolving types within the logical formulas
//!    that govern the action's applicability (e.g., in quantifiers).
//! 3. **Effect Resolution**: Resolving types within the formulas that describe
//!    state transitions, ensuring all modified fluents and quantified effects
//!    remain type-consistent.

use crate::aiplan4rust::lir::ActionDef;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::passes::either_type::expr;
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::lir::passes::either_type::TypeRegistry;
use crate::aiplan4rust::lir::passes::either_type::typed_list;

/// Flattens a primitive action definition in-place.
///
/// This function simplifies the types within the action's parameters and
/// propagates those changes through both the precondition and effect
/// expression trees.
///
/// # Parameters
/// * `action` - A mutable reference to the [`ActionDef`] to be transformed.
/// * `registry` - The [`TypeRegistry`] used to unify and resolve type signatures.
/// * `stack` - A reusable [`Vec<NodeId>`] buffer used for efficient, non-recursive
///   depth-first traversal of the expression trees.
///
/// # Returns
/// * `Ok(())` if the action was successfully flattened.
/// * `Err(LirError)` if parameter resolution or expression tree traversal fails.
///
/// # Implementation Detail
/// By reusing the same `registry` and `stack` across both preconditions and effects,
/// this function ensures that variable references remain consistent within the
/// action's scope while minimizing memory allocations.
pub fn flatten(
    action: &mut ActionDef,
    registry: &mut TypeRegistry,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    // 1. Flatten the action's parameters (the signature).
    // This resolves any 'either' types into canonical atomic identifiers.
    typed_list::flatten_typed_variable_list(action.parameters_mut(), registry)?;

    // 2. Flatten the precondition expression tree.
    // Reuses the pre-allocated stack to avoid unnecessary memory overhead during traversal.
    expr::flatten(action.precondition_mut(), registry, stack)?;

    // 3. Flatten the effect expression tree.
    // The registry ensures that any quantified variables in the effects match
    // the global flattened type system.
    expr::flatten(action.effect_mut(), registry, stack)?;

    Ok(())
}
