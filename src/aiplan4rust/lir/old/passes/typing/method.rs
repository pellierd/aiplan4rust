//! # HTN Method Normalization
//!
//! This module implements the type resolution and normalization logic for HTN Methods
//! within the Lifted IR (LIR).
//!
//! ## Overview
//! A Method in a Hierarchical Task Network defines the decomposition of an abstract
//! task into a sub-network of tasks. To ensure the grounder can process these
//! decompositions efficiently, this pass unifies all composite type signatures
//! within the method's scope into atomic identifiers.
//!
//! The transformation follows a two-step process:
//! 1. **Parameter Normalization**: Resolving types in the method's signature to ensure
//!    variable declarations use unified atomic identifiers.
//! 2. **Precondition Normalization**: Recursively traversing the precondition
//!    expression tree to resolve type signatures within logical constraints
//!    (specifically within `Exists` and `Forall` quantifiers).
//!
//! This ensures that both the "interface" (parameters) and the "logic" (preconditions)
//! of the method are perfectly aligned with the flattened type hierarchy.

use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::lir::old::passes::typing::TypeRegistry;
use crate::aiplan4rust::lir::old::passes::typing::{expr, typed_list};
use crate::aiplan4rust::lir::MethodDef;
use crate::aiplan4rust::tree::NodeId;

/// Normalizes an HTN method definition in-place.
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
/// * `Ok(())` if both parameters and preconditions were successfully normalized.
/// * `Err(LirError)` if a type resolution error occurs during the traversal.
///
/// # Implementation Detail
/// The use of a mutable `stack` buffer prevents unnecessary allocations during
/// the traversal of deep logical formulas, improving performance during the
/// normalization phase of large planning problems.
pub fn normalize(
    method: &mut MethodDef,
    registry: &mut TypeRegistry,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    // 1. Process the primary source of types: the method's parameter list.
    // This ensures the method's signature is consistent with the flattened domain.
    typed_list::normalize_typed_variable_list(method.parameters_mut(), registry)?;

    // 2. Process the precondition expression tree.
    // The registry resolves types found in quantifiers, while the stack facilitates
    // the structural visit of the tree.
    expr::normalize(method.precondition_mut(), registry, stack)?;

    Ok(())
}
