//! HTN Task Network Encoding
//!
//! This module provides functionality to encode Hierarchical Task Networks (HTN).
//! It transforms syntax subtrees into a structured `TaskNetwork` by resolving
//! subtasks, ordering dependencies, and logical constraints against the LIR context.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{expr, EncodingRegistry};
use crate::aiplan4rust::lir::problem::task_network::TaskNetwork;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::SyntaxSubtree;

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
pub fn encode(
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
}
