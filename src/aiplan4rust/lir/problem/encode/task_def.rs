//! HTN Abstract Task Encoding
//!
//! This module handles the encoding and registration of abstract (compound) tasks.
//!
//! In HTN planning, an abstract task represents a high-level objective. It
//! acts as a contract: any method claiming to decompose this task must
//! match its signature.
//!
//! This module specializes a generic [`NamedTypedList`] into a [`Task`]
//! skeleton, stores it in the LIR, and updates the registry to map the
//! task's symbol node to its internal ID.

use crate::aiplan4rust::lir::atomic_skeleton::task::Task;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::problem::encode::{named_typed_list, EncodingRegistry};
use crate::aiplan4rust::lir::problem::LiftedProblem;

/// Encodes an HTN abstract task signature and registers it within the LIR context.
///
/// This function translates an AST task definition into its Lifted representation by:
/// 1. **Parsing**: Extracting the task name and its typed parameters using [`named_typed_list`].
/// 2. **LIR Storage**: Adding the resulting [`Task`] skeleton to the [`LiftedProblem`].
/// 3. **Registry Binding**: Mapping the task's name `NodeId` to both its [`TaskID`]
///    and [`TaskSkeletonID`] in the [`EncodingRegistry`].
///
/// This dual registration allows the encoder to resolve task references in methods
/// by linking the specific AST symbol to its resolved LIR counterparts.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the task (e.g., `(transport ?p - package)`).
/// * `registry` - The mutable registry for symbol resolution and ID mapping.
/// * `ir` - The mutable [`LiftedProblem`] where the task is stored.
///
/// # Errors
///
/// Returns [`LirError`] if the signature is malformed, types cannot be resolved,
/// or mandatory components are missing.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    // 1. Reuse the generic signature encoder (Name + Parameters)
    let signature = named_typed_list::encode(subtree, registry)?;

    // 2. Specialized Task creation
    let task_skeleton = Task::from_header(signature);

    // 3. Identify the Task Name NodeId
    // We bind the specific symbol node (e.g., 'transport') to the LIR ID to
    // match how the symbol table resolves task references in methods.
    let task_symbol_node_id = subtree.node().children()[0];

    // 4. Dual registration: Physical storage and Node-based mapping
    let (task_symbol_id, task_skeleton_id) = ir.add_task_skeleton(task_skeleton);
    registry.register_task_skeleton(task_symbol_node_id, task_skeleton_id);
    registry.register_task_symbol(task_symbol_node_id, task_symbol_id);

    Ok(())
}
