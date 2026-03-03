//! Goal Condition Encoding
//!
//! This module handles the translation of the PDDL `:goal` section.
//! It transforms the goal's logical requirements into a LIR expression
//! that must be satisfied in any valid plan prefix or final state.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::encoding::{expr, EncodingRegistry};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encodes the goal condition of the problem from the `:goal` AST section.
///
/// This function extracts the logical root of the goal specification and delegates
/// its recursive encoding to the expression module. It ensures that all symbols
/// (predicates, constants, variables) within the goal are correctly resolved.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree corresponding to the `Goal` node.
/// * `evaluator` - The evaluator used for symbol lookup and scoping.
/// * `ir` - The mutable lifted problem where goal-related bindings are registered.
///
/// # Returns
///
/// * `Ok(Expr)` - The encoded logical expression representing the goal.
/// * `Err(LirError)` - If the goal structure is malformed or symbol resolution fails.
///
/// # Errors
///
/// This function returns an error if:
/// * The mandatory child node representing the goal's ops is missing.
/// * The underlying expression fails to encoding (e.g., unknown predicate).
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<Expr, LirError> {
    // 1. Access the first child of the Goal node (the root of the logical expression)
    let child_id = subtree.node().try_child(0)?;
    let child_node = subtree.tree().try_node(child_id)?;
    let child_subtree = SyntaxSubtree::new(child_node, child_id, subtree.tree());

    // 2. Encode using the context to resolve symbols (objects, predicates, etc.)
    // We pass ir mutably to register bindings if necessary.
    expr::encode(&child_subtree, registry)
}
