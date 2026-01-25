//! Initial State Encoding
//!
//! This module handles the translation of the PDDL `:init` section.
//! It transforms the initial facts and assignments defined in the problem
//! file into LIR expressions.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{expr, EncodingContext};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;

/// Encodes the initial state of the problem from the `:init` AST section.
///
/// This function extracts the collection of ground facts and fluents. It delegates
/// the recursive encoding to the expression module, ensuring all initial predicates
/// and functions are correctly bound to the problem's symbol table.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree corresponding to the `Init` node.
/// * `ctx` - The encoding context for symbol and index resolution.
/// * `ir` - The mutable lifted problem where the initial state is registered.
///
/// # Returns
///
/// * `Ok(Expr)` - An expression representing the conjunctive initial state.
/// * `Err(LirError)` - If the initial state structure is invalid or contains
///   unresolved symbols.
///
/// # Errors
///
/// This function returns an error if the mandatory child node (containing the
/// list of facts) is missing or cannot be parsed as a valid expression.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    ctx: &EncodingContext,
    ir: &mut LiftedProblem,
) -> Result<Expr, LirError> {
    // 1. Access the first child of the Init node (containing the list of initial facts)
    let child_id = subtree.node().try_child(0)?;
    let child_node = subtree.tree().try_node(child_id)?;
    let child_subtree = SyntaxSubtree::new(child_node, child_id, subtree.tree());

    // 2. Use the free expression encoder to transform the AST into a LIR Expr.
    // This populates the problem's binding tables (predicate_bindings).
    expr::encode(&child_subtree, ctx, ir)
}
