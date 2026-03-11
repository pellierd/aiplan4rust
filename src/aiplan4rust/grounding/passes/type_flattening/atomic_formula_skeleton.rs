//! # Atomic Formula Flattening
//!
//! This module implements the type flattening logic for Atomic Formulae (Predicates).
//!
//! ## Overview
//! Atomic Formulae are the building blocks of logical conditions in PDDL and HDN.
//! They consist of a predicate symbol applied to a list of typed arguments.
//!
//! Flattening an atomic formula ensures that all its arguments are restricted 
//! to primitive types. This is essential for the *grounding* process, as it 
//! allows the system to match predicate arguments against a flat set of 
//! objects and constants.

use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::grounding::passes::type_flattening::typed_list;
use crate::type_flattening::PivotTracker;

/// Flattens an atomic formula skeleton in-place.
///
/// This function simplifies the types of the predicate's parameters. It 
/// delegates the iteration and mutation of the parameter list to the 
/// specialized [`typed_list`] module.
///
/// # Arguments
/// * `atomic_formula` - A mutable reference to the predicate skeleton to transform.
/// * `tracker` - The shared [`PivotTracker`] used to map complex types to primitive pivots.
///
/// # Errors
/// Returns a [`LirError`] if the parameter list transformation fails.
pub fn flatten(
    atomic_formula: &mut AtomicFormulaSkeleton,
    tracker: &mut PivotTracker,
) -> Result<(), LirError> {
    // Delegate the flattening of the parameter list to the typed_list module.
    // This transforms any 'either' types in the predicate's signature into 
    // canonical primitive types.
    typed_list::flatten_typed_variable_list(atomic_formula.parameters_mut(), tracker)
}
