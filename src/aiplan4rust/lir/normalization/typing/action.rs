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
//!    root types into unified atomic identifiers via in-place mutation.
//! 2. **Precondition Normalization**: Resolving types within the logical formulas
//!    that govern the action's applicability (e.g., in quantifiers).
//! 3. **Effect Normalization**: Resolving types within the formulas that describe
//!    state transitions, ensuring all modified fluents and quantified effects
//!    remain type-consistent.
//!
//! ## Memory Optimization
//! This module enforces zero runtime allocations by reusing a centralized [`Scratchpad`]
//! across both precondition and effect expression trees, avoiding heap churn during
//! the normalization of large domains.

use crate::aiplan4rust::lir::expr::iter::Scratchpad;
use crate::aiplan4rust::lir::expr::ExprStore;
use crate::aiplan4rust::lir::normalization::error::NormalizationError;
use crate::aiplan4rust::lir::normalization::typing::{expr, typed_list, TypeRegistry};
use crate::aiplan4rust::lir::problem::ActionDef;

/// Normalizes a primitive action definition in-place.
///
/// This function simplifies the types within the action's parameters and
/// propagates those changes through both the precondition and effect
/// expression trees hosted inside the provided [`ExprStore`].
///
/// # Arguments
/// * `action` - A mutable reference to the [`ActionDef`] to be transformed.
/// * `store` - A mutable reference to the global expression store hosting the node entries.
/// * `registry` - A mutable reference to the [`TypeRegistry`] used to unify and resolve type signatures.
/// * `scratch` - An external, reusable memory arena tracking the traversal state, buffers, and ID translation cache.
///
/// # Returns
/// * `Ok(())` if the action was successfully normalized.
/// * `Err(NormalizationError)` if parameter resolution or expression tree traversal fails.
///
/// # Implementation Detail
/// By reusing the same `store`, `registry`, and `scratch` instances across both preconditions and effects,
/// this function ensures that variable references remain consistent within the
/// action's scope while guaranteeing a strict **$\mathcal{O}(1)$ dynamic allocation profile**.
pub fn normalize(
    action: &mut ActionDef,
    store: &mut ExprStore,
    registry: &mut TypeRegistry,
    scratch: &mut Scratchpad,
) -> Result<(), NormalizationError> {
    // 1. Normalize the action's parameters (the signature) in-place.
    // This reduces compound types (e.g., 'either') into primitive, atomic type identifiers.
    typed_list::normalize_typed_variable_list(action.parameters_mut(), registry)?;

    // 2. Normalize the precondition expression tree.
    let normalized_precondition = expr::normalize(action.precondition(), store, registry, scratch)?;
    action.set_precondition(normalized_precondition);

    // 3. Normalize the effect expression tree.
    let normalized_effect = expr::normalize(action.effect(), store, registry, scratch)?;
    action.set_effect(normalized_effect);

    Ok(())
}
