//! # Atomic Formula Flattening
//!
//! This module implements the type resolution and flattening logic for Atomic
//! Formulae (Predicates) within the LIR.
//!
//! ## Overview
//! Atomic Formulae are the fundamental building blocks of logical conditions in
//! PDDL and HTN models. They consist of a predicate symbol applied to a list of
//! typed arguments.
//!
//! Flattening an atomic formula ensures that all its arguments are resolved into
//! unified atomic types. This is a prerequisite for the **grounding** process,
//! as it allows the engine to efficiently match predicate arguments against a
//! standardized set of objects and constants without managing type unions.

use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::passes::either_type::typed_list;
use crate::aiplan4rust::lir::passes::either_type::TypeRegistry;

/// Flattens an atomic formula skeleton in-place.
///
/// This function simplifies the type signatures of the predicate's parameters.
/// It delegates the iteration and mutation of the parameter list to the
/// specialized [`typed_list`] module.
///
/// # Parameters
/// * `atomic_formula` - A mutable reference to the [`AtomicFormulaSkeleton`] to transform.
/// * `registry` - The [`TypeRegistry`] used to resolve and unify composite type signatures.
///
/// # Returns
/// * `Ok(())` if the predicate's parameter list was successfully flattened.
/// * `Err(LirError)` if the parameter list transformation encounters a registry error.
///
/// # Logic
/// By transforming any `either` types in the predicate's signature into
/// unified atomic `TypeId`s, this function ensures that all calls to this
/// predicate across the problem (in preconditions or effects) refer to the
/// same canonical type definitions.
pub fn flatten(
    atomic_formula: &mut AtomicFormulaSkeleton,
    registry: &mut TypeRegistry,
) -> Result<(), LirError> {
    // Delegate the flattening of the parameter list.
    // This transforms composite types into atomic identifiers in-place.
    typed_list::flatten_typed_variable_list(atomic_formula.parameters_mut(), registry)
}
