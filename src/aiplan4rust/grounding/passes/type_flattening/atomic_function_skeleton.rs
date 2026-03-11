//! # Atomic Function Flattening
//!
//! This module implements the type flattening logic for Atomic Functions (Fluents).
//!
//! ## Overview
//! Atomic Functions map a set of typed parameters to a specific return type. 
//! To ensure the entire domain is flattened, both the input signature and 
//! the output type must be transformed.
//!
//! The process involves:
//! 1. **Parameter Flattening**: Simplifying the types of the function's 
//!    arguments using the [`typed_list`] utility.
//! 2. **Return Type Flattening**: Simplifying the result type of the function 
//!    using the core [`ty`] logic.

use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::grounding::passes::type_flattening::{ty, typed_list};
use crate::type_flattening::PivotTracker;

/// Flattens an atomic function skeleton in-place.
///
/// This function ensures that any hierarchical or union types present in 
/// either the parameters or the return type are replaced by primitive 
/// pivot types.
///
/// # Arguments
/// * `atomic_function` - A mutable reference to the function skeleton to transform.
/// * `tracker` - The shared [`PivotTracker`] for consistent type mapping.
///
/// # Errors
/// Returns a [`LirError`] if the parameter list or the return type 
/// transformation fails.
pub fn flatten(
    atomic_function: &mut AtomicFunctionSkeleton,
    tracker: &mut PivotTracker,
) -> Result<(), LirError> {
    // 1. Flatten the input parameters (the signature).
    // This ensures that function calls in expressions will match the flattened types.
    typed_list::flatten_typed_variable_list(atomic_function.parameters_mut(), tracker)?;

    // 2. Flatten the return type.
    // Since functions return values, the return type itself might be an 'either' type 
    // that needs to be collapsed into a pivot type.
    ty::flatten(atomic_function.ty_mut(), tracker)?;

    Ok(())
}
