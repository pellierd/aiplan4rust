//! # Typed List Flattening
//!
//! This module provides functions to process collections of typed symbols, 
//! such as parameter lists for actions, methods, or tasks.
//!
//! ## Overview
//! It acts as an iterator wrapper, ensuring that every element within a 
//! [`TypedList`] is consistently transformed. By iterating through the list 
//! and delegating the logic to [`typed_symbol`], it maintains the integrity 
//! of the entire variable set according to the [`PivotTracker`].

use crate::aiplan4rust::lang::{TypeId, TypedList, VariableId};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::grounding::passes::type_flattening::typed_symbol;
use crate::type_flattening::PivotTracker;

/// Flattens all variables within a typed list.
///
/// This is typically used to simplify the parameter lists of actions or tasks.
/// It iterates mutably over the list and applies the flattening logic to each 
/// variable symbol.
///
/// # Arguments
/// * `typed_list` - A mutable reference to the list of variables to transform.
/// * `tracker` - The pivot tracker used to manage type substitutions.
///
/// # Errors
/// Returns a [`LirError`] if any individual variable transformation fails.
pub fn flatten_typed_variable_list(
    typed_list: &mut TypedList<VariableId, TypeId>,
    tracker: &mut PivotTracker,
) -> Result<(), LirError> {
    // Iterate over each TypedSymbol in the list
    for ts in typed_list.iter_mut() {
        // Delegate to the specialized variable flattening function
        typed_symbol::flatten_typed_variable(ts, tracker)?;
    }

    Ok(())
}
