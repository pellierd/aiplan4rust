//! # Typed Symbol Flattening
//!
//! This module provides specialized functions to flatten the type signatures of symbols
//! within the LIR, specifically targeting objects (constants) and variables.
//!
//! ## Overview
//! Since [`TypedSymbol`] is a generic container, these functions handle the
//! extraction of mutable type references and delegate the core resolution
//! logic to the [`ty`] module.
//!
//! When a symbol's type is composite (e.g., an `either` type), it is resolved
//! into a unified atomic `TypeId`. This transformation is managed and tracked
//! by the [`TypeRegistry`] to ensure consistency across the entire problem.

use crate::aiplan4rust::lang::{ObjectId, TypeId, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::passes::either_type::TypeRegistry;
use crate::aiplan4rust::lir::passes::either_type::ty;

/// Flattens the type of a constant object.
///
/// This function accesses the mutable type signature of a `TypedSymbol<ObjectId, TypeId>`
/// and resolves it using the provided registry.
///
/// # Parameters
/// * `symbol` - A mutable reference to the typed object symbol to transform.
/// * `registry` - The [`TypeRegistry`] used to unify and resolve type signatures.
///
/// # Returns
/// * `Ok(())` upon successful type resolution.
/// * `Err(LirError)` if the type transformation violates LIR structural constraints.
pub fn flatten_typed_object(
    symbol: &mut TypedSymbol<ObjectId, TypeId>,
    registry: &mut TypeRegistry,
) -> Result<(), LirError> {
    // 1. Gain mutable access to the object's type
    let ty = symbol.ty_mut();

    // 2. Delegate the transformation.
    // Composite types are unified and replaced with a single atomic TypeId.
    ty::flatten(ty, registry)
}

/// Flattens the type of a variable.
///
/// This function extracts the mutable type from a `TypedSymbol<VariableId, TypeId>`
/// and simplifies it. This is typically used for action parameters, method
/// variables, or quantified variables (Exists/Forall).
///
/// # Parameters
/// * `symbol` - A mutable reference to the typed variable symbol to transform.
/// * `registry` - The [`TypeRegistry`] used to unify and resolve type signatures.
///
/// # Returns
/// * `Ok(())` if the variable's type was successfully flattened.
/// * `Err(LirError)` if the variable type cannot be resolved within the registry.
pub fn flatten_typed_variable(
    symbol: &mut TypedSymbol<VariableId, TypeId>,
    registry: &mut TypeRegistry,
) -> Result<(), LirError> {
    // 1. Gain mutable access to the variable's type
    let ty = symbol.ty_mut();

    // 2. Delegate the transformation to the type module.
    // If 'ty' represents a composite type, it is transformed into an atomic TypeId in-place.
    ty::flatten(ty, registry)
}
