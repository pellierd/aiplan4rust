//! # Typed Symbol Flattening
//!
//! This module provides specialized functions to flatten the types of symbols 
//! used in the LIR, specifically for objects (constants) and variables.
//!
//! ## Overview
//! Since `TypedSymbol` is a generic container, these functions handle the 
//! extraction of the mutable type reference and delegate the actual 
//! flattening logic to the [`ty`] module.
//!
//! If a symbol's type is a complex type (like an `either` type), it will be 
//! reduced to a primitive "pivot" type, with the transformation tracked 
//! by the [`PivotTracker`].

use crate::aiplan4rust::lang::{ObjectId, TypeId, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::LirError;
use crate::type_flattening::PivotTracker;
use crate::type_flattening::ty;

/// Flattens the type of a constant object.
///
/// This function extracts the mutable type from a `TypedSymbol<ObjectId, TypeId>` 
/// and simplifies it using the provided tracker.
///
/// # Errors
/// Returns a [`LirError`] if the type transformation violates LIR constraints.
pub fn flatten_typed_object(
    symbol: &mut TypedSymbol<ObjectId, TypeId>,
    tracker: &mut PivotTracker,
) -> Result<(), LirError> {
    // 1. Gain mutable access to the object's type
    let ty = symbol.ty_mut();

    // 2. Delegate the transformation.
    // Complex types (like 'either') are replaced with a primitive 'pivot' type.
    ty::flatten(ty, tracker)
}

/// Flattens the type of a variable.
///
/// This function extracts the mutable type from a `TypedSymbol<VariableId, TypeId>` 
/// and simplifies it. This is typically used for action parameters or 
/// quantified variables.
///
/// # Errors
/// Returns a [`LirError`] if the variable type cannot be flattened correctly.
pub fn flatten_typed_variable(
    symbol: &mut TypedSymbol<VariableId, TypeId>,
    tracker: &mut PivotTracker,
) -> Result<(), LirError> {
    // 1. Gain mutable access to the variable's type
    let ty = symbol.ty_mut();

    // 2. Delegate the transformation to the type module via the tracker.
    // If 'ty' is a complex type, it is transformed into a primitive type in-place.
    ty::flatten(ty, tracker)
}
