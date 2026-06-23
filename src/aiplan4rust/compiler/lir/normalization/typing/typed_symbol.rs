//! # Typed Symbol Normalization
//!
//! This module provides specialized functions to normalize the type signatures of symbols
//! within the Lifted IR (LIR), specifically targeting objects (constants) and variables.
//!
//! ## Overview
//! Since [`TypedSymbol`] is a generic container used throughout the lifted representation,
//! these functions handle the extraction of mutable type references and delegate the support
//! atomic resolution logic to the [`ty`] module.
//!
//! By ensuring that every [`TypedSymbol`] (whether it's an action parameter or a constant)
//! has a single, non-composite [`TypeId`], we guarantee that the grounder can perform
//! direct mapping without complex type checks.
//!
//! The transformation of composite types (e.g., `either`) into unified atomic identifiers
//! is consistent across the problem thanks to the shared [`TypeRegistry`].

use crate::aiplan4rust::compiler::lir::normalization::error::NormalizationError;
use crate::aiplan4rust::compiler::lir::normalization::typing::ty;
use crate::aiplan4rust::compiler::lir::normalization::typing::TypeRegistry;
use crate::aiplan4rust::support::lang::{ObjectId, TypeId, TypedSymbol, VariableId};

/// Normalizes the type of a constant object in-place.
///
/// This function accesses the mutable type signature of a `TypedSymbol<ObjectId, TypeId>`
/// (representing a PDDL constant or object) and resolves it to an atomic identifier.
///
/// # Parameters
/// * `symbol` - A mutable reference to the typed object symbol to transform.
/// * `registry` - The [`TypeRegistry`] used to unify and resolve type signatures.
///
/// # Returns
/// * `Ok(())` upon successful type resolution.
/// * `Err(LirError)` if the type transformation violates LIR structural constraints.
pub fn normalize_typed_object(
    symbol: &TypedSymbol<ObjectId, TypeId>,
    registry: &mut TypeRegistry,
) -> Result<TypedSymbol<ObjectId, TypeId>, NormalizationError> {
    let norm_ty = ty::normalize(symbol.ty(), registry)?;
    // On reconstruit le symbole avec son symbole original et le nouveau type normalisé
    Ok(TypedSymbol::new(symbol.symbol(), norm_ty))
}

/// Normalizes the type of a variable in-place.
///
/// This function extracts the mutable type from a `TypedSymbol<VariableId, TypeId>`
/// and simplifies it. This is essential for action parameters, method
/// variables, or quantified variables (Exists/Forall) before grounding.
///
/// # Parameters
/// * `symbol` - A mutable reference to the typed variable symbol to transform.
/// * `registry` - The [`TypeRegistry`] used to unify and resolve type signatures.
///
/// # Returns
/// * `Ok(())` if the variable's type was successfully normalized.
/// * `Err(LirError)` if the variable type cannot be resolved within the registry.
pub fn normalize_typed_variable(
    symbol: &TypedSymbol<VariableId, TypeId>,
    registry: &mut TypeRegistry,
) -> Result<TypedSymbol<VariableId, TypeId>, NormalizationError> {
    let norm_ty = ty::normalize(symbol.ty(), registry)?;
    Ok(TypedSymbol::new(symbol.symbol(), norm_ty))
}
