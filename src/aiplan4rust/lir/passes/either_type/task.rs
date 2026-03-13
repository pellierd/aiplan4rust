//! # HTN Task Flattening
//!
//! This module handles the structural simplification of Abstract Tasks within
//! the Hierarchical Task Network (HTN) hierarchy.
//!
//! ## Overview
//! In the LIR, Abstract Tasks serve as symbolic headers for decomposition. Unlike
//! actions or methods, they do not contain logic bodies, preconditions, or effects;
//! they are defined solely by their signature (parameters).
//!
//! This pass ensures that any composite types (e.g., `either` types) used in
//! a task's parameter list are resolved into unified atomic types. This maintains
//! consistency between the tasks and the methods that decompose them.

use crate::aiplan4rust::lir::problem::atomic_skeleton::task::Task;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::passes::either_type::typed_list;
use crate::aiplan4rust::lir::passes::either_type::TypeRegistry;

/// Flattens the parameters of an HTN abstract task.
///
/// This function performs an in-place mutation of the task's parameter list.
/// It identifies composite type signatures and replaces them with the unified
/// atomic `TypeId`s managed by the [`TypeRegistry`].
///
/// # Parameters
/// * `task` - A mutable reference to the [`Task`] (Abstract Task) to be transformed.
/// * `registry` - The [`TypeRegistry`] used to resolve and unify type signatures
///   across the problem.
///
/// # Returns
/// * `Ok(())` if the task's parameters were successfully flattened.
/// * `Err(LirError)` if the parameter list transformation encounters a registry error.
///
/// # Logic
/// Since Abstract Tasks are purely structural, this function delegates the
/// flattening of the typed variable list to the [`typed_list`] module and
/// does not need to visit any expression trees.
pub fn flatten(
    task: &mut Task,
    registry: &mut TypeRegistry,
) -> Result<(), LirError> {
    // Abstract tasks consist only of a name and parameters.
    // We only need to resolve types within the parameter list.
    typed_list::flatten_typed_variable_list(task.parameters_mut(), registry)
}
