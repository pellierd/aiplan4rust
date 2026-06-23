//! # Derived Predicate Normalization
//!
//! This module implements the type resolution and normalization logic for Derived
//! Predicates (also known as Axioms) within the Lifted IR (LIR).
//!
//! ## Overview
//! A Derived Predicate consists of a **head** (its signature/skeleton) and a **body**
//! (a logical expression defining the conditions under which the predicate holds).
//!
//! Normalizing a derived predicate involves:
//! 1. **Head Resolution**: Updating the types of the predicate's parameters
//!    to use unified atomic identifiers via in-place mutation.
//! 2. **Body Resolution**: Reconstructing the logical formula that defines the predicate,
//!    ensuring all internal variables and quantifiers are consistent with the simplified
//!    type registry.
//!
//! This ensures that when the grounder evaluates an axiom, it doesn't encounter
//! complex type unions, significantly simplifying the stratified evaluation and inference engine.
//!
//! ## Memory Optimization
//! This module enforces zero runtime allocations by reusing a centralized [`Scratchpad`]
//! to execute an iterative post-order traversal over the body expression tree,
//! avoiding stack overflows and heap churn.

use crate::aiplan4rust::compiler::lir::expr::iter::Scratchpad;
use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::normalization::typing::expr;
use crate::aiplan4rust::compiler::lir::normalization::typing::skeleton::formula;
use crate::aiplan4rust::compiler::lir::normalization::typing::TypeRegistry;
use crate::aiplan4rust::compiler::lir::normalization::NormalizationError;
use crate::aiplan4rust::compiler::lir::problem::derived_predicate::DerivedPredicate;

/// Normalizes a derived predicate definition in-place.
///
/// This function simplifies the types within the predicate's signature (head)
/// and propagates those changes throughout the logical definition (body) of the axiom
/// hosted inside the provided [`ExprStore`].
///
/// # Arguments
/// * `derived_predicate` - A mutable reference to the [`DerivedPredicate`] to transform.
/// * `old` - A mutable reference to the global expression old hosting the node entries.
/// * `registry` - A mutable reference to the [`TypeRegistry`] used to unify and resolve type signatures.
/// * `pad` - An external, reusable memory arena tracking the traversal state, buffers, and ID translation cache.
///
/// # Returns
/// * `Ok(())` if both the head and the body were successfully normalized.
/// * `Err(NormalizationError)` if the signature normalization or the expression tree traversal
///   encounters a structural inconsistency or registry error.
///
/// # Implementation Detail
/// The function leverages the specialized [`skeleton::formula`] sub-module logic for the head, as
/// derived predicate signatures share the same structural requirements as standard atomic predicates.
/// By extracting the copyable [`ExprId`] representing the body before processing, it safely isolates
/// borrows and respects a strict **$\mathcal{O}(1)$ dynamic allocation profile**.
pub fn normalize(
    derived_predicate: &mut DerivedPredicate,
    store: &mut ExprStore,
    registry: &mut TypeRegistry,
    pad: &mut Scratchpad,
) -> Result<(), NormalizationError> {
    // 1. Normalize the signature (the "head") en passant le store.
    // Cela va mettre à jour le TypedListId interne de `derived_predicate.head`.
    formula::normalize(derived_predicate.head_mut(), store, registry)?;

    // 2. Normalize the logical definition (the "body").
    let normalized_body = expr::normalize(derived_predicate.body(), store, registry, pad)?;
    derived_predicate.set_body(normalized_body);

    Ok(())
}
