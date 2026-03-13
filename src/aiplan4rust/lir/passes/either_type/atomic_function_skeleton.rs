//! # Atomic Function Flattening
//!
//! This module implements the type resolution and flattening logic for Atomic
//! Functions (Fluents) within the LIR.
//!
//! ## Overview
//! Atomic Functions map a specific set of typed parameters to a return type.
//! To maintain consistency throughout the flattened domain, both the input
//! signature and the output type must be resolved into unified atomic identifiers.
//!
//! The transformation process involves:
//! 1. **Parameter Flattening**: Resolving the types of the function's arguments
//!    using the [`typed_list`] utility.
//! 2. **Return Type Flattening**: Resolving the function's result type using
//!    the core [`ty`] logic.

use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::passes::either_type::{ty, typed_list};
use crate::aiplan4rust::lir::passes::either_type::TypeRegistry;

/// Flattens an atomic function skeleton in-place.
///
/// This function ensures that any composite types (e.g., `either` types) present
/// in the parameters or the return type are replaced by unified atomic `TypeId`s
/// managed by the [`TypeRegistry`].
///
/// # Parameters
/// * `atomic_function` - A mutable reference to the [`AtomicFunctionSkeleton`] to transform.
/// * `registry` - The [`TypeRegistry`] used to unify and resolve type signatures.
///
/// # Returns
/// * `Ok(())` if both the parameters and the return type were successfully flattened.
/// * `Err(LirError)` if the parameter list or the return type transformation
///   encounters a registry resolution error.
///
/// # Logic
/// Functions are dual-faceted: they act as predicates in their signature but
/// as terms in their return value. This function ensures that both facets
/// are normalized, preventing type mismatches when the function is evaluated
/// within an expression.
pub fn flatten(
    atomic_function: &mut AtomicFunctionSkeleton,
    registry: &mut TypeRegistry,
) -> Result<(), LirError> {
    // 1. Flatten the input parameters (the signature).
    // This ensures that function applications in expressions match the flattened
    // domain types.
    typed_list::flatten_typed_variable_list(atomic_function.parameters_mut(), registry)?;

    // 2. Flatten the return type.
    // In many planning domains, functions may return values of composite types.
    // We collapse these into a single unified atomic TypeId.
    ty::flatten(atomic_function.ty_mut(), registry)?;

    Ok(())
}
