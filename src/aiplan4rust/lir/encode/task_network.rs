//! HTN Task Network Encoding
//!
//! This module provides functionality to encode Hierarchical Task Networks (HTN).
//! It transforms syntax subtrees into a structured `TaskNetwork` by resolving
//! subtasks, ordering dependencies, and logical constraints against the LIR context.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{StringID, TaskLabelID, TaskSkeletonID};
use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprKind};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::expr::kind::Kind;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::encode::{expr, EncodingRegistry};
use crate::aiplan4rust::lir::task_network::TaskNetwork;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::{NodeId, SyntaxSubtree};

/// Encodes a HTN Task Network from the given syntax subtree.
///
/// This function parses subtask definitions (ordered or partially ordered),
/// task ordering constraints, and logical constraints to build a `TaskNetwork`.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the task network definition.
/// * `registry` - The registry used for symbol and identifier resolution.
/// * `ir` - The mutable lifted problem used to register or reference LIR elements.
///
/// # Returns
///
/// * `Ok(TaskNetwork)` - A fully encoded task network ready for HTN planning.
/// * `Err(LirError)` - If the AST contains unexpected nodes or if expression encoding fails.
///
/// # Errors
///
/// This function will return an error if:
/// * A child node kind is not recognized as a valid task network component.



/// Main entry point for Task Network encoding.
/// Orchestrates the two-pass process: 1. ID Collection, 2. Content Encoding.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<TaskNetwork, LirError> {
    // 0. Reset local scope to prevent cross-method label leakage
    registry.clear_task_label();

    // PASS 1: Scan for labels (t1:, t2:) and map them to indices
    collect_task_labels(subtree, registry)?;

    // PASS 2: Perform the actual expression encoding
    encode_task_network_content(subtree, registry)
}

/// PASS 1: Scans the network to register task labels into the registry using a match pattern.
fn collect_task_labels(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<(), LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    for &child_id in node.children() {
        let child_node = ast.try_node(child_id)?;

        match child_node.kind() {
            // We only care about subtask definitions for ID collection
            AstKind::PartiallyOrderedSubtaskDef | AstKind::OrderedSubtaskDef => {
                let tasks_node_id = child_node.try_child(0)?;
                let tasks_node = ast.try_node(tasks_node_id)?;

                for &tagged_task_node_id in tasks_node.children() {
                    let tagged_task_node = ast.try_node(tagged_task_node_id)?;
                    let tag_node_id = tagged_task_node.try_child(0)?;
                    let tag_node = ast.try_node(tag_node_id)?;
                    registry.register_task_label(tag_node.try_ident()?);
                }
            }
            // Other nodes (ordering, constraints) are ignored in this pass
            _ => {}
        }
    }
    Ok(())
}

/// PASS 2: Encodes tasks, ordering, and constraints into LIR expressions.
fn encode_task_network_content(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<TaskNetwork, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    let mut tasks = Expr::empty_and();
    let mut ordering = Expr::empty_and();
    let mut constraints = Expr::empty_and();
    let mut total_ordered = false;

    for &child_id in node.children() {
        let child_node = ast.try_node(child_id)?;

        match child_node.kind() {
            AstKind::PartiallyOrderedSubtaskDef => {
                let tasks_node_id = child_node.try_child(0)?;
                let tasks_node = ast.try_node(tasks_node_id)?;
                tasks = expr::encode(&SyntaxSubtree::new(tasks_node, tasks_node_id, ast), registry)?;
            }
            AstKind::OrderedSubtaskDef => {
                let tasks_node_id = child_node.try_child(0)?;
                let tasks_node = ast.try_node(tasks_node_id)?;
                tasks = expr::encode(&SyntaxSubtree::new(tasks_node, tasks_node_id, ast), registry)?;
                total_ordered = true;
            }
            AstKind::TaskOrderingConstraintDef => {
                let ordering_node_id = child_node.try_child(0)?;
                let ordering_node = ast.try_node(ordering_node_id)?;
                ordering = expr::encode(&SyntaxSubtree::new(ordering_node, ordering_node_id, ast), registry)?;
            }
            AstKind::TaskLogicalConstraintDef => {
                let logical_node_id = child_node.try_child(0)?;
                let logical_node = ast.try_node(logical_node_id)?;
                constraints = expr::encode(&SyntaxSubtree::new(logical_node, logical_node_id, ast), registry)?;
            }
            _ => return Err(LirError::task_network_ast_kind_error(child_node.kind())),
        }
    }
    finalize_task_network(tasks, ordering, constraints, total_ordered, registry)
}


fn finalize_task_network(
    tasks: Expr,
    ordering: Expr,
    constraints: Expr,
    total_ordered: bool,
    registry: &EncodingRegistry,
) -> Result<TaskNetwork, LirError> {
    let num_tasks = registry.task_label_count();
    let mut task_nodes = vec![NodeId::default(); num_tasks];
    let mut task_defs = vec![TaskSkeletonID::default(); num_tasks];
    let mut task_labels = vec![StringID::default(); num_tasks];

    // On parcourt tous les nœuds de l'expression LIR
    for node in tasks.preorder().values() {
        if node.kind() == ExprKind::TaggedTask {
            let task_label_node_id = node.try_child(0)?;
            let task_label_node = tasks.try_node(task_label_node_id)?;

            if let ExprContent::TaskID(label_id) = task_label_node.content() {
                let index = label_id.as_usize();
                task_labels[index] = registry.resolve_task_label_symbol(*label_id);

                let task_node_id = node.try_child(1)?;
                task_nodes[index] = task_node_id;
                let task_node = tasks.try_node(task_node_id)?;

                if let ExprContent::TaskSkeleton(task_skeleton_id) = task_node.content() {
                    task_defs[index] = *task_skeleton_id;
                }
            }
        }
    }
    Ok(TaskNetwork::new(tasks, ordering, constraints, total_ordered, task_labels, task_defs, task_nodes))
}

/*pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<TaskNetwork, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    let mut tasks = Expr::empty_and();
    let mut ordering = Expr::empty_and();
    let mut constraints = Expr::empty_and();
    let mut is_declared_total_ordered = false;

    for &child_id in node.children() {
        let child_node = ast.try_node(child_id)?;

        match child_node.kind() {
            AstKind::PartiallyOrderedSubtaskDef => {
                let tasks_node_id = child_node.try_child(0)?;
                let tasks_node = ast.try_node(tasks_node_id)?;
                tasks = expr::encode(&SyntaxSubtree::new(tasks_node, tasks_node_id, ast), registry)?;
            }
            AstKind::OrderedSubtaskDef => {
                let tasks_node_id = child_node.try_child(0)?;
                let tasks_node = ast.try_node(tasks_node_id)?;
                tasks = expr::encode(&SyntaxSubtree::new(tasks_node, tasks_node_id, ast), registry)?;
                is_declared_total_ordered = true;
            }
            AstKind::TaskOrderingConstraintDef => {
                let ordering_node_id = child_node.try_child(0)?;
                let ordering_node = ast.try_node(ordering_node_id)?;
                ordering = expr::encode(&SyntaxSubtree::new(ordering_node, ordering_node_id, ast), registry)?;
            }
            AstKind::TaskLogicalConstraintDef => {
                let logical_node_id = child_node.try_child(0)?;
                let logical_node = ast.try_node(logical_node_id)?;
                constraints = expr::encode(&SyntaxSubtree::new(logical_node, logical_node_id, ast), registry)?;
            }
            _ => {
                return Err(LirError::task_network_ast_kind_error(child_node.kind()));
            }
        }
    }

    Ok(TaskNetwork::new(
        tasks,
        ordering,
        constraints,
        is_declared_total_ordered,
    ))
}*/
