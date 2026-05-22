//! # Atomic Formula Normalization
//!
//! This module implements the type resolution and normalization logic for Atomic
//! Formulae (Predicates) within the Lifted IR (LIR).
//!
//! ## Overview
//! Atomic Formulae are the fundamental building blocks of logical conditions in
//! PDDL and HTN models. They consist of a predicate symbol applied to a list of
//! typed arguments.
//!
//! Normalizing an atomic formula ensures that all its arguments are resolved into
//! unified atomic identifiers. This is a prerequisite for the **grounding** process,
//! as it allows the engine to efficiently match predicate arguments against a
//! standardized set of objects and constants without managing complex type unions
//! at runtime.

use crate::aiplan4rust::lir::store::passes::typing::typed_list;
use crate::aiplan4rust::lir::store::passes::typing::TypeRegistry;
use crate::aiplan4rust::lir::store::problem_old::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::LirError;

/// Normalizes an atomic formula skeleton in-place.
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
/// * `Ok(())` if the predicate's parameter list was successfully normalized.
/// * `Err(LirError)` if the parameter list transformation encounters a registry error.
///
/// # Logic
/// By transforming any `either` types in the predicate's signature into
/// unified atomic [`TypeId`]s, this function ensures that all calls to this
/// predicate across the problem (in preconditions or effects) refer to the
/// same canonical type definitions. This creates a uniform interface for the
/// grounding engine.
pub fn normalize(
    atomic_formula: &mut AtomicFormulaSkeleton,
    registry: &mut TypeRegistry,
) -> Result<(), LirError> {
    // Delegate the normalization of the parameter list.
    // This transforms composite types or root types into atomic identifiers in-place.
    typed_list::normalize_typed_variable_list(atomic_formula.parameters_mut(), registry)
}
