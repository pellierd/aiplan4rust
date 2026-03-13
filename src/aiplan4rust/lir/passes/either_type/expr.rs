//! # Expression Tree Flattening
//!
//! This module implements a non-recursive, stack-based traversal to resolve and
//! unify type signatures within logical expression trees and quantifier scopes.
//!
//! ## Memory Optimization
//! To ensure high performance and prevent stack overflows on deep or complex
//! expression trees, this module avoids recursion. It utilizes an external
//! [`Vec<NodeId>`] as a reusable stack buffer, minimizing heap reallocations
//! when processing large domains with hundreds of actions or methods.
//!
//! ## Quantifier Handling
//! The primary objective of this module is resolving composite `either` types within
//! `Forall` and `Exists` quantifiers. When a quantified variable is identified with
//! a composite type, it is mutated in-place to a unified atomic `TypeId` managed
//! by the [`TypeRegistry`].

use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::lir::passes::either_type::{typed_symbol, TypeRegistry};

/// Entry point for expression tree flattening.
///
/// This function initializes a depth-first search (DFS) traversal of the expression
/// tree. It accepts an external stack buffer to allow for efficient reuse across
/// multiple calls during the normalization pipeline.
///
/// # Parameters
/// * `expr` - A mutable reference to the [`Expr`] tree to be flattened.
/// * `registry` - The [`TypeRegistry`] used to unify and resolve type signatures.
/// * `stack` - A mutable [`Vec<NodeId>`] buffer used for non-recursive traversal.
///
/// # Returns
/// * `Ok(())` if the expression tree was successfully traversed and resolved.
/// * `Err(LirError)` if the tree structure is invalid or type resolution fails.
pub fn flatten(
    expr: &mut Expr,
    registry: &mut TypeRegistry,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    if expr.is_empty() {
        return Ok(());
    }

    let root_id = expr.try_root_id()?;

    // Clear the stack to ensure a clean state before starting the new traversal.
    stack.clear();

    flatten_from_node(expr, root_id, registry, stack)
}

/// Performs a non-recursive depth-first traversal starting from a specific node.
///
/// This function handles the mutation of quantified variables while navigating
/// the expression tree. It employs a scoped borrow pattern to satisfy the
/// Rust borrow checker while performing in-place mutations on the tree nodes.
///
/// # Parameters
/// * `expr` - A mutable reference to the [`Expr`] tree.
/// * `node_id` - The identifier of the node where the traversal starts.
/// * `registry` - The [`TypeRegistry`] used for type unification.
/// * `stack` - The reusable navigation stack.
///
/// # Mechanism
/// 1. **Mutation Phase**: If the current node is a quantifier (`Forall` or `Exists`),
///    its variable list is mutated to replace composite types with atomic IDs.
/// 2. **Traversal Phase**: The children of the current node are pushed onto the
///    stack to continue the DFS.
///
/// # Errors
/// Returns a [`LirError`] if node access fails or if quantifier variables
/// are missing expected type data.
pub fn flatten_from_node(
    expr: &mut Expr,
    node_id: NodeId,
    registry: &mut TypeRegistry,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {
    stack.push(node_id);

    while let Some(current_id) = stack.pop() {
        // --- STEP 1: MUTATION (Scoped Borrow) ---
        // Block scope is used to drop the mutable borrow of 'expr' before
        // accessing the children IDs in the next step.
        {
            let node = expr.try_node_mut(current_id)?;

            if matches!(node.kind(), ExprKind::Forall | ExprKind::Exists) {
                let vars = node.content_mut().try_quantifier_vars_mut()?;

                for var in vars.iter_mut() {
                    // Resolve the type of each quantified variable.
                    typed_symbol::flatten_typed_variable(var, registry)?;
                }
            }
        } // <--- Mutable borrow is released here.

        // --- STEP 2: TRAVERSAL (Read-only Access) ---
        // After releasing the mutable borrow, we can safely iterate over the children.
        let children = expr.try_node(current_id)?.children();
        for &child_id in children {
            stack.push(child_id);
        }
    }

    Ok(())
}
