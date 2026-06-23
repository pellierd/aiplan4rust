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

use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::normalization::error::NormalizationError;
use crate::aiplan4rust::compiler::lir::normalization::typing::{typed_list, TypeRegistry};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;

/// Normalizes an atomic formula skeleton in-place.
///
/// This function simplifies the type signatures of the predicate's parameters.
/// It delegates the iteration and mutation of the parameter list to the
/// specialized [`typed_list`] module.
///
/// # Arguments
/// * `atomic_formula` - A mutable reference to the [`AtomicFormulaSkeleton`] to transform.
/// * `registry` - A mutable reference to the [`TypeRegistry`] used to resolve and unify composite type signatures.
///
/// # Returns
/// * `Ok(())` if the predicate's parameter list was successfully normalized.
/// * `Err(NormalizationError)` if the parameter list transformation encounters a registry error.
///
/// # Logic
/// By transforming any `either` types in the predicate's signature into
/// unified atomic [`TypeId`]s, this function ensures that all calls to this
/// predicate across the problem (in preconditions or effects) refer to the
/// same canonical type definitions. This creates a uniform interface for the
/// grounding engine while maintaining a strict **$\mathcal{O}(1)$ dynamic allocation profile**.
pub fn normalize(
    atomic_formula: &mut AtomicFormulaSkeleton,
    store: &mut ExprStore,
    registry: &mut TypeRegistry,
) -> Result<(), NormalizationError> {
    // 1. On prend l'ID actuel des paramètres de la formule
    let current_param_id = atomic_formula.parameters();

    // 2. On délègue au normalisateur de liste qui travaille avec le store
    let normalized_param_id =
        typed_list::normalize_typed_variable_list(current_param_id, store, registry)?;

    // 3. On met à jour l'ID des paramètres dans la formule skeleton
    atomic_formula.set_parameters(normalized_param_id);

    Ok(())
}
