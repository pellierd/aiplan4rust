//! # Derived Predicate Flattening
//!
//! This module implements the type resolution and flattening logic for Derived
//! Predicates (also known as Axioms).
//!
//! ## Overview
//! A Derived Predicate consists of a **head** (its signature/skeleton) and a **body** //! (a logical expression defining the conditions under which the predicate holds).
//!
//! Flattening a derived predicate involves:
//! 1. **Head Resolution**: Updating the types of the predicate's parameters
//!    to use unified atomic identifiers.
//! 2. **Body Resolution**: Recursively updating the logical formula that
//!    defines the predicate, ensuring all internal variables and quantifiers
//!    are consistent with the simplified type registry.

use crate::aiplan4rust::lir::problem::derived_predicate::DerivedPredicate;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::passes::either_type::{atomic_formula_skeleton, expr};
use crate::aiplan4rust::lir::passes::either_type::TypeRegistry;
use crate::aiplan4rust::tree::NodeId;

/// Flattens a derived predicate definition in-place.
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
/// * `Ok(())` if both the head and the body were successfully flattened.
/// * `Err(LirError)` if the signature flattening or the expression tree traversal
///   encounters a structural inconsistency or registry error.
///
/// # Logic
/// The function leverages the [`atomic_formula_skeleton`] logic for the head, as
/// derived predicate signatures share the same structural requirements as standard
/// predicates. The body is processed via the standard expression visitor.
pub fn flatten(
    derived_predicate: &mut DerivedPredicate,
    registry: &mut TypeRegistry,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    // 1. Flatten the signature (the "head").
    // Derived predicate heads are structurally identical to standard predicates,
    // allowing us to reuse the atomic formula resolution logic.
    atomic_formula_skeleton::flatten(derived_predicate.head_mut(), registry)?;

    // 2. Flatten the logical definition (the "body").
    // We utilize the provided stack to perform an efficient, non-recursive DFS
    // traversal of the expression tree, resolving any types within quantifier scopes.
    expr::flatten(derived_predicate.body_mut(), registry, stack)?;

    Ok(())
}
