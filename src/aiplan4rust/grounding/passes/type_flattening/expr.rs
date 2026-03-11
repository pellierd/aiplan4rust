//! # Expression Tree Flattening
//!
//! This module implements a non-recursive, stack-based traversal to flatten 
//! types within logical expressions and quantifier scopes.
//!
//! ## Memory Optimization
//! To ensure high performance and prevent stack overflow on deep expression 
//! trees, this module avoids recursion. It utilizes an external `Vec<NodeId>` 
//! as a reusable stack buffer, minimizing heap reallocations during the 
//! grounding pipeline.
//!
//! ## Quantifier Handling
//! The primary focus of this module is resolving `either` types within 
//! `forall` and `exists` quantifiers. When a quantified variable is found 
//! with a complex type, it is mutated in-place to a primitive pivot type.

use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::tree::NodeId;
use crate::type_flattening::PivotTracker;

/// Entry point for expression flattening.
///
/// This function initializes the traversal. It accepts an external stack 
/// to allow for buffer reuse across multiple calls (e.g., when processing 
/// hundreds of actions).
///
/// # Arguments
/// * `expr` - The expression tree to be flattened.
/// * `tracker` - The shared [`PivotTracker`] for type ID mapping.
/// * `stack` - A mutable buffer used for the depth-first search (DFS) traversal.
pub fn flatten(
    expr: &mut Expr,
    tracker: &mut PivotTracker,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    if expr.is_empty() {
        return Ok(());
    }

    let root_id = expr.try_root_id()?;

    // Clear the stack to ensure a clean state before starting the new traversal.
    stack.clear();

    flatten_from_node(expr, root_id, tracker, stack)
}

/// Performs a depth-first traversal starting from a specific node.
///
/// This function handles the mutation of quantified variables while 
/// navigating the expression tree. It uses a scoped borrow pattern to 
/// satisfy the Rust borrow checker during in-place mutations.
///
/// # Mechanism
/// 1. **Mutation Phase**: If the node is a quantifier, its variables are 
///    mutated to primitive types using the tracker.
/// 2. **Traversal Phase**: The children of the current node are pushed 
///    onto the stack for subsequent processing.
pub fn flatten_from_node(
    expr: &mut Expr,
    node_id: NodeId,
    tracker: &mut PivotTracker,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    stack.push(node_id);

    while let Some(current_id) = stack.pop() {
        // --- STEP 1: MUTATION (Scoped) ---
        // We use a block scope to ensure the mutable borrow of 'expr' 
        // is dropped before we attempt to read the children IDs.
        {
            let node = expr.try_node_mut(current_id)?;

            if matches!(node.kind(), ExprKind::Forall | ExprKind::Exists) {
                let vars = node.content_mut().try_quantifier_vars_mut()?;

                for var in vars.iter_mut() {
                    if var.ty().is_either() {
                        // Retrieve the pivot ID and update the variable's type.
                        let pivot_id = tracker.next_type_id(var.ty().members());
                        var.set_ty(Type::primitive(pivot_id));
                    }
                }
            }
        } // <--- Mutable borrow is released here.

        // --- STEP 2: TRAVERSAL (Read-only) ---
        // With the mutable borrow gone, we can safely access the children list.
        let children = expr.try_node(current_id)?.children();
        for &child_id in children {
            stack.push(child_id);
        }
    }

    Ok(())
}
