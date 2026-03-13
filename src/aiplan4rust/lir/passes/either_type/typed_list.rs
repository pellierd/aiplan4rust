//! # Typed List Flattening
//!
//! This module provides functions to process collections of typed symbols, 
//! such as parameter lists for actions, methods, or tasks.
//!
//! ## Overview
//! It acts as a structural visitor, ensuring that every element within a
//! [`TypedList`] is consistently transformed. By iterating through the list
//! and delegating the logic to [`typed_symbol`], it maintains the integrity
//! of the entire variable set according to the [`TypeRegistry`].

use crate::aiplan4rust::lang::{TypeId, TypedList, VariableId};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::passes::either_type::typed_symbol;
use crate::aiplan4rust::lir::passes::either_type::TypeRegistry;

/// Flattens all variables within a typed list using the provided registry.
///
/// This function is primarily used to simplify the parameter lists of actions,
/// tasks, or methods. It performs an in-place mutation of the list, resolving
/// any composite types into their unified atomic equivalents.
///
/// # Parameters
/// * `typed_list` - A mutable reference to the list of variables ([`TypedList`]) to transform.
/// * `registry` - The [`TypeRegistry`] used to resolve and unify type signatures.
///
/// # Returns
/// * `Ok(())` if the entire list was successfully processed.
/// * `Err(LirError)` if a variable transformation encountered an inconsistency.
///
/// # Logic
/// The function iterates mutably over the list and delegates the flattening of
/// each individual [`TypedSymbol`] to the specialized variable flattening module.
pub fn flatten_typed_variable_list(
    typed_list: &mut TypedList<VariableId, TypeId>,
    registry: &mut TypeRegistry,
) -> Result<(), LirError> {
    // Iterate over each TypedSymbol in the list
    for ts in typed_list.iter_mut() {
        // Delegate to the specialized variable flattening function
        typed_symbol::flatten_typed_variable(ts, registry)?;
    }

    Ok(())
}
