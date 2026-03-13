//! # HTN Method Flattening
//!
//! This module implements the type resolution and flattening logic for HTN Methods.
//!
//! ## Overview
//! A Method in a Hierarchical Task Network defines the decomposition of an abstract
//! task into a sub-network of tasks. To ensure the grounder can process these
//! decompositions efficiently, this pass unifies all composite type signatures
//! within the method's scope.
//!
//! The transformation follows a two-step process:
//! 1. **Parameter Flattening**: Resolving types in the method's signature to ensure
//!    variable declarations use unified atomic identifiers.
//! 2. **Precondition Flattening**: Recursively traversing the precondition
//!    expression tree to resolve type signatures within logical constraints
//!    (specifically within `Exists` and `Forall` quantifiers).

use crate::aiplan4rust::lir::MethodDef;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::passes::either_type::{expr, typed_list};
use crate::aiplan4rust::lir::passes::either_type::TypeRegistry;
use crate::aiplan4rust::tree::NodeId;

/// Flattens an HTN method definition in-place.
///
/// This function simplifies the type signatures within the method's parameters and
/// propagates these changes throughout the precondition expression tree.
///
/// # Parameters
/// * `method` - A mutable reference to the [`MethodDef`] to be transformed.
/// * `registry` - The [`TypeRegistry`] used to unify and resolve type signatures.
/// * `stack` - A reusable [`Vec<NodeId>`] buffer used for efficient, non-recursive
///   depth-first traversal of the precondition expression tree.
///
/// # Returns
/// * `Ok(())` if both parameters and preconditions were successfully flattened.
/// * `Err(LirError)` if a type resolution error occurs during the traversal.
///
/// # Implementation Detail
/// The use of a mutable `stack` buffer prevents unnecessary allocations during
/// the traversal of deep logical formulas, improving performance during the
/// normalization phase of large planning problems.
pub fn flatten(
    method: &mut MethodDef,
    registry: &mut TypeRegistry,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    // 1. Process the primary source of types: the method's parameter list.
    // This ensures the method's signature is consistent with the flattened domain.
    typed_list::flatten_typed_variable_list(method.parameters_mut(), registry)?;

    // 2. Process the precondition expression tree.
    // The registry resolves types found in quantifiers, while the stack facilitates
    // the structural visit of the tree.
    expr::flatten(method.precondition_mut(), registry, stack)?;

    Ok(())
}
