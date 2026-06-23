//! # Typed List Normalization
//!
//! This module provides functions to process collections of typed symbols within the
//! Lifted IR (LIR), such as parameter lists for actions, methods, or tasks.
//!
//! ## Overview
//! It acts as a structural visitor, ensuring that every element within a [`TypedList`]
//! is consistently transformed. By iterating through the list and delegating the
//! normalization logic to [`typed_symbol`], it maintains the integrity of entire
//! variable sets according to the [`TypeRegistry`].
//!
//! This step is crucial for the grounder, as it ensures that the domains of all
//! parameters are clearly defined by atomic type identifiers.

use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::normalization::error::NormalizationError;
use crate::aiplan4rust::compiler::lir::normalization::typing::{typed_symbol, TypeRegistry};
use crate::aiplan4rust::support::lang::{TypedList, TypedListId};

/// Normalizes all variables within a typed list in-place.
///
/// This function is primarily used to simplify the parameter lists of lifted
/// constructs (actions, tasks, or methods). It performs an in-place mutation,
/// resolving any composite `either` types or missing root types into their
/// unified atomic equivalents.
///
/// # Parameters
/// * `typed_list` - A mutable reference to the list of variables ([`TypedList`]) to transform.
/// * `registry` - The [`TypeRegistry`] used to resolve and unify type signatures.
///
/// # Returns
/// * `Ok(())` if the entire list was successfully processed.
/// * `Err(ExprOpErrorHC)` if a variable transformation encountered a resolution error.
///
/// # Logic
/// The function iterates mutably over the list and delegates the normalization of
/// each individual [`TypedSymbol`] to the specialized variable normalization module.
/// This ensures that the entire "signature" of a lifted operator is grounder-ready.
pub fn normalize_typed_variable_list(
    list_id: TypedListId,
    store: &mut ExprStore,
    registry: &mut TypeRegistry,
) -> Result<TypedListId, NormalizationError> {
    // Si la liste est invalide/absente (NONE) ou vide (EMPTY), on la retourne directement
    if list_id.is_none() {
        return Ok(TypedListId::NONE);
    }
    if list_id.is_empty() {
        return Ok(TypedListId::EMPTY);
    }

    // 1. Extraction de la liste immuable depuis l'arène du store
    let original_list = store.fetch_typed_list(list_id)?;

    // 2. Création d'une nouvelle liste de travail locale
    let mut normalized_list = TypedList::with_capacity(original_list.len());

    // 3. Normalisation de chaque symbole de variable
    for ts in original_list.iter() {
        let norm_ts = typed_symbol::normalize_typed_variable(ts, registry)?;
        normalized_list.push(norm_ts);
    }

    // 4. L'étape cruciale du Hash-Consing
    let new_list_id = store.intern_typed_list(normalized_list);

    Ok(new_list_id)
}
