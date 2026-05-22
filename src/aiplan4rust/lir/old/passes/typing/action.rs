//! # Action Definition Normalization
//!
//! This module implements the type resolution and normalization logic for primitive actions
//! within the Lifted IR (LIR).
//!
//! ## Overview
//! Actions are the fundamental operators of a planning domain. Normalizing
//! an action requires a consistent transformation of its signature and its
//! logical components to ensure that the grounded state space remains valid and
//! computationally efficient.
//!
//! The transformation process follows a three-step pipeline:
//! 1. **Signature Normalization**: Converting composite parameter types or missing
//!    root types into unified atomic identifiers.
//! 2. **Precondition Normalization**: Resolving types within the logical formulas
//!    that govern the action's applicability (e.g., in quantifiers).
//! 3. **Effect Normalization**: Resolving types within the formulas that describe
//!    state transitions, ensuring all modified fluents and quantified effects
//!    remain type-consistent.
//!
//! This unified approach guarantees that the action's entire scope—from its interface
//! to its internal dynamics—is ready for the grounding engine.

use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::old::passes::typing::expr;
use crate::aiplan4rust::lir::old::passes::typing::typed_list;
use crate::aiplan4rust::lir::old::passes::typing::TypeRegistry;
use crate::aiplan4rust::lir::ActionDef;
use crate::aiplan4rust::tree::NodeId;

/// Normalizes a primitive action definition in-place.
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
/// * `Ok(())` if the action was successfully normalized.
/// * `Err(LirError)` if parameter resolution or expression tree traversal fails.
///
/// # Implementation Detail
/// By reusing the same `registry` and `stack` across both preconditions and effects,
/// this function ensures that variable references remain consistent within the
/// action's scope while minimizing memory allocations.
pub fn normalize(
    action: &mut ActionDef,
    registry: &mut TypeRegistry,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    // 1. Normalize the action's parameters (the signature).
    // This resolves any 'either' types into canonical atomic identifiers.
    typed_list::normalize_typed_variable_list(action.parameters_mut(), registry)?;

    // 2. Normalize the precondition expression tree.
    // Reuses the pre-allocated stack to avoid unnecessary memory overhead during traversal.
    expr::normalize(action.precondition_mut(), registry, stack)?;

    // 3. Normalize the effect expression tree.
    // The registry ensures that any quantified variables in the effects match
    // the global normalized type system.
    expr::normalize(action.effect_mut(), registry, stack)?;

    Ok(())
}
