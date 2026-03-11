//! # Initial Task Network (ITN) Flattening
//!
//! This module handles the type simplification for the problem's entry point.
//!
//! ## Overview
//! The Initial Task Network defines the top-level variables and tasks that 
//! the planner must resolve. If these variables use hierarchical or union 
//! types, they must be flattened to ensure the initial state of the 
//! search space is compatible with the flattened domain.

use crate::aiplan4rust::lir::InitialTaskNetwork;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::grounding::passes::type_flattening::typed_list;
use crate::type_flattening::PivotTracker;

/// Flattens all union types (`Type::Either`) within an `InitialTaskNetwork` in-place.
///
/// This function remaps the types of the initial task network parameters to 
/// match the primitive pivot types generated during the flattening pass.
///
/// # Arguments
/// * `itn` - A mutable reference to the `InitialTaskNetwork` structure to modify.
/// * `tracker` - The shared [`PivotTracker`] containing the mapping from 
///   union types to their unique flattened primitive `TypeId`.
///
/// # Errors
/// * Returns a [`LirError`] if a union type cannot be resolved or if the 
///   parameter transformation fails.
pub fn flatten(
    itn: &mut InitialTaskNetwork,
    tracker: &mut PivotTracker,
) -> Result<(), LirError> {
    // 1. Flatten the Initial Task Network parameters.
    // We transform 'either' types into pivot types for variables declared 
    // at the problem level. This is the "entry point" of the HTN search.
    typed_list::flatten_typed_variable_list(itn.parameters_mut(), tracker)
}
