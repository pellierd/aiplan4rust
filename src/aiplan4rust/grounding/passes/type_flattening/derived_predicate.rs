//! # Derived Predicate Flattening
//!
//! This module implements the type flattening logic for Derived Predicates (Axioms).
//!
//! ## Overview
//! A Derived Predicate consists of a **head** (its signature) and a **body** //! (a logical expression defining when the predicate is true). 
//!
//! Flattening a derived predicate involves:
//! 1. **Head Flattening**: Updating the types of the predicate's parameters 
//!    to use primitive pivot types.
//! 2. **Body Flattening**: Recursively updating the logical formula that 
//!    defines the predicate, ensuring all internal variables and quantifiers 
//!    are consistent with the new type system.

use crate::aiplan4rust::lir::problem::derived_predicate::DerivedPredicate;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::grounding::passes::type_flattening::{atomic_formula_skeleton, expr};
use crate::type_flattening::PivotTracker;
use crate::aiplan4rust::tree::NodeId;

/// Flattens a derived predicate definition in-place.
///
/// This function simplifies the types within the predicate's signature (head) 
/// and propagates those changes through the logical definition (body).
///
/// # Arguments
/// * `derived_predicate` - A mutable reference to the predicate to transform.
/// * `tracker` - The shared [`PivotTracker`] for consistent type mapping.
/// * `stack` - A reusable buffer for the non-recursive traversal of the body expression.
///
/// # Errors
/// Returns a [`LirError`] if the signature flattening or the expression 
/// tree traversal encounters an inconsistency.
pub fn flatten(
    derived_predicate: &mut DerivedPredicate,
    tracker: &mut PivotTracker,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    // 1. Flatten the signature (the "head").
    // We reuse the atomic formula logic as derived predicate heads share 
    // the same structure as standard predicates.
    atomic_formula_skeleton::flatten(derived_predicate.head_mut(), tracker)?;

    // 2. Flatten the logical definition (the "body").
    // We utilize the provided stack to perform an efficient DFS traversal 
    // of the expression tree, which may contain complex logical operators.
    expr::flatten(derived_predicate.body_mut(), tracker, stack)?;

    Ok(())
}
