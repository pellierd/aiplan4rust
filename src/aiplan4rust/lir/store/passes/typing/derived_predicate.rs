//! # Derived Predicate Normalization
//!
//! This module implements the type resolution and normalization logic for Derived
//! Predicates (also known as Axioms) within the Lifted IR (LIR).
//!
//! ## Overview
//! A Derived Predicate consists of a **head** (its signature/skeleton) and a **body** //! (a logical expression defining the conditions under which the predicate holds).
//!
//! Normalizing a derived predicate involves:
//! 1. **Head Resolution**: Updating the types of the predicate's parameters
//!    to use unified atomic identifiers.
//! 2. **Body Resolution**: Recursively updating the logical formula that
//!    defines the predicate, ensuring all internal variables and quantifiers
//!    are consistent with the simplified type registry.
//!
//! This ensures that when the grounder evaluates an axiom, it doesn't encounter
//! complex type unions, significantly simplifying the inference engine.

use crate::aiplan4rust::lir::store::passes::typing::TypeRegistry;
use crate::aiplan4rust::lir::store::passes::typing::{atomic_formula_skeleton, expr};
use crate::aiplan4rust::lir::store::problem_old::derived_predicate::DerivedPredicate;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::tree::NodeId;

/// Normalizes a derived predicate definition in-place.
///
/// This function simplifies the types within the predicate's signature (head)
/// and propagates those changes throughout the logical definition (body) of the axiom.
///
/// # Parameters
/// * `derived_predicate` - A mutable reference to the [`DerivedPredicate`] to transform.
/// * `registry` - The [`TypeRegistry`] used to unify and resolve type signatures.
/// * `stack` - A reusable [`Vec<NodeId>`] buffer used for efficient, non-recursive
///   depth-first traversal of the body expression tree.
///
/// # Returns
/// * `Ok(())` if both the head and the body were successfully normalized.
/// * `Err(LirError)` if the signature normalization or the expression tree traversal
///   encounters a structural inconsistency or registry error.
///
/// # Logic
/// The function leverages the [`atomic_formula_skeleton`] logic for the head, as
/// derived predicate signatures share the same structural requirements as standard
/// predicates. The body is processed via the standard expression visitor.
pub fn normalize(
    derived_predicate: &mut DerivedPredicate,
    registry: &mut TypeRegistry,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    // 1. Normalize the signature (the "head").
    // Derived predicate heads are structurally identical to standard predicates,
    // allowing us to reuse the atomic formula resolution logic.
    atomic_formula_skeleton::normalize(derived_predicate.head_mut(), registry)?;

    // 2. Normalize the logical definition (the "body").
    // We utilize the provided stack to perform an efficient, non-recursive DFS
    // traversal of the expression tree, resolving any types within quantifier scopes.
    expr::normalize(derived_predicate.body_mut(), registry, stack)?;

    Ok(())
}
