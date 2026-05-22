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

use crate::aiplan4rust::lang::{TypeId, TypedList, VariableId};
use crate::aiplan4rust::lir::old::passes::typing::typed_symbol;
use crate::aiplan4rust::lir::old::passes::typing::TypeRegistry;
use crate::aiplan4rust::lir::LirError;

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
/// * `Err(LirError)` if a variable transformation encountered a resolution error.
///
/// # Logic
/// The function iterates mutably over the list and delegates the normalization of
/// each individual [`TypedSymbol`] to the specialized variable normalization module.
/// This ensures that the entire "signature" of a lifted operator is grounder-ready.
pub fn normalize_typed_variable_list(
    typed_list: &mut TypedList<VariableId, TypeId>,
    registry: &mut TypeRegistry,
) -> Result<(), LirError> {
    // Iterate over each TypedSymbol in the list
    for ts in typed_list.iter_mut() {
        // Delegate to the specialized variable normalization function
        typed_symbol::normalize_typed_variable(ts, registry)?;
    }

    Ok(())
}
