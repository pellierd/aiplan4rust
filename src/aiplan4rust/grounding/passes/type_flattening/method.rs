//! # HTN Method Flattening
//!
//! This module implements the type flattening logic for HTN Methods.
//!
//! ## Overview
//! A Method in HTN defines how an abstract task can be decomposed into a 
//! sub-network of tasks. Flattening a method requires ensuring that its 
//! signature and its applicability conditions (preconditions) use 
//! consistent primitive types.
//!
//! The process involves:
//! 1. **Parameter Flattening**: Simplifying the types of the method's variables.
//! 2. **Precondition Flattening**: Recursively traversing the precondition 
//!    expression tree to resolve hierarchical types in logical constraints 
//!    (e.g., in `forall` or `exists` quantifiers).

use crate::aiplan4rust::lir::MethodDef;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::grounding::passes::type_flattening::{expr, typed_list};
use crate::type_flattening::PivotTracker;
use crate::aiplan4rust::tree::NodeId;

/// Flattens an HTN method definition in-place.
///
/// This function simplifies the types within the method's parameters and 
/// propagates those changes through the precondition expression tree.
///
/// # Arguments
/// * `method` - A mutable reference to the Method definition to transform.
/// * `tracker` - The shared pivot tracker for consistent type mapping.
/// * `stack` - A reusable stack buffer for the depth-first traversal of 
///   the precondition expression tree.
///
/// # Errors
/// Returns a [`LirError`] if parameter flattening or expression 
/// traversal fails.
pub fn flatten(
    method: &mut MethodDef,
    tracker: &mut PivotTracker,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    // 1. Process the primary source of types: the method's parameter list.
    // This ensures the method's signature matches the flattened domain.
    typed_list::flatten_typed_variable_list(method.parameters_mut(), tracker)?;

    // 2. Process the precondition expression tree.
    // We pass the stack to allow for an efficient, non-recursive 
    // traversal of the logical formula.
    expr::flatten(method.precondition_mut(), tracker, stack)?;

    Ok(())
}
