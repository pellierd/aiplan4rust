//! # HTN Task Flattening
//!
//! This module handles the simplification of Abstract Tasks within the HTN 
//! hierarchy.
//!
//! ## Overview
//! Unlike actions or methods, tasks in the LIR do not contain logic bodies or 
//! preconditions; they are defined by their symbolic signature (parameters). 
//!
//! This pass ensures that any complex types (e.g., `either` types) used in 
//! a task's parameter list are replaced by their corresponding primitive 
//! pivot types to maintain consistency across the task network.

use crate::aiplan4rust::lir::problem::atomic_skeleton::task::Task;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::grounding::passes::type_flattening::typed_list;
use crate::type_flattening::PivotTracker;

/// Flattens the parameters of an HTN task.
///
/// This function mutates the task's parameter list in-place. It identifies 
/// hierarchical types and replaces them with flattened versions tracked 
/// by the [`PivotTracker`].
///
/// # Arguments
/// * `task` - A mutable reference to the Abstract Task to be transformed.
/// * `tracker` - The shared pivot tracker for consistent type mapping.
///
/// # Errors
/// Returns a [`LirError`] if the parameter list transformation fails.
pub fn flatten(
    task: &mut Task,
    tracker: &mut PivotTracker,
) -> Result<(), LirError> {
    // HTN tasks do not have expression bodies or preconditions.
    // We only need to flatten the parameters which may contain 'either' types.
    typed_list::flatten_typed_variable_list(task.parameters_mut(), tracker)
}
