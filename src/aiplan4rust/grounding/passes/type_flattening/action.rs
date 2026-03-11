//! # Action Definition Flattening
//!
//! This module implements the type flattening logic for primitive actions.
//!
//! ## Overview
//! Actions are the fundamental operators of a planning domain. Flattening 
//! an action requires a consistent transformation of its signature and its 
//! logical components to ensure that the grounded state space remains valid.
//!
//! The transformation process follows three steps:
//! 1. **Signature Flattening**: Converting hierarchical parameter types into 
//!    primitive pivot types.
//! 2. **Precondition Flattening**: Resolving types within the logical formulas 
//!    that govern the action's applicability.
//! 3. **Effect Flattening**: Resolving types within the formulas that describe 
//!    how the world state changes.

use crate::aiplan4rust::lir::ActionDef;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::grounding::passes::type_flattening::{expr};
use crate::aiplan4rust::tree::NodeId;
use crate::type_flattening::pivot_tracker::PivotTracker;
use crate::type_flattening::typed_list;

/// Flattens a primitive action definition in-place.
///
/// This function simplifies the types within the action's parameters and 
/// propagates those changes through both the precondition and effect 
/// expression trees.
///
/// # Arguments
/// * `action` - A mutable reference to the Action definition to transform.
/// * `tracker` - The shared [`PivotTracker`] for consistent type mapping.
/// * `stack` - A reusable buffer for the non-recursive traversal of the 
///   expression trees, preventing frequent heap allocations.
///
/// # Errors
/// Returns a [`LirError`] if parameter flattening or expression 
/// traversal fails in either the preconditions or effects.
pub fn flatten(
    action: &mut ActionDef,
    tracker: &mut PivotTracker,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    // 1. Flatten the action's parameters (the signature).
    // This centralizes the logic for variable-type mapping.
    typed_list::flatten_typed_variable_list(action.parameters_mut(), tracker)?;

    // 2. Flatten the precondition expression tree.
    // Reuses the pre-allocated stack to avoid unnecessary memory overhead.
    expr::flatten(action.precondition_mut(), tracker, stack)?;

    // 3. Flatten the effect expression tree.
    // The same tracker and stack are used to ensure the entire action 
    // is consistent with the global flattened domain.
    expr::flatten(action.effect_mut(), tracker, stack)?;

    Ok(())
}
