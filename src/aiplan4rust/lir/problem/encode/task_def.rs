//! HTN Abstract Task Encoding
//!
//! This module handles the encoding of abstract (compound) tasks.
//! In HTN planning, a task defines a high-level objective that can be
//! decomposed into subtasks through various methods.

use crate::aiplan4rust::lir::atomic_skeleton::task::Task;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::problem::encode::named_typed_list;

/// Encodes an HTN abstract task from the syntax tree.
///
/// This function extracts the task's signature (name and parameters).
/// Unlike actions, abstract tasks do not have direct preconditions or effects,
/// as their logic is defined by the methods that decompose them.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the task definition.
///
/// # Returns
///
/// * `Ok(Task)` - The encoded task header.
/// * `Err(LirError)` - If the signature (name or parameters) is malformed.
///
/// # Errors
///
/// This function returns an error if `named_typed_list::encode` fails to
/// parse the task name or its typed parameter list.
pub fn encode(subtree: &SyntaxSubtree<AstNode>) -> Result<Task, LirError> {
    // 1. Encode the task signature (name + parameters)
    // Abstract tasks are defined by their header in the domain
    let signature = named_typed_list::encode(subtree)?;

    Ok(Task::from_header(signature))
}
