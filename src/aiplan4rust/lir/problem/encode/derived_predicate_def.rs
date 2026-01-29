//! Derived Predicate Encoding
//!
//! This module handles the encoding of PDDL derived predicates (axioms).
//! Derived predicates allow the domain to define new relations based on
//! existing ones, which are automatically updated as the state changes.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::derived_predicate::DerivedPredicate;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::problem::encode::{atomic_formula_skeleton, expr};
use crate::aiplan4rust::lir::problem::encode::registry::EncodingContext;

/// Encodes a derived predicate from the syntax tree into the LIR.
///
/// This function translates a PDDL axiom by encoding its "head" (the predicate
/// being defined) and its "body" (the logical condition that makes it true).
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `:derived` definition.
/// * `ctx` - The encoding context for resolving identifiers within the logic.
/// * `ir` - The mutable `LiftedProblem` where the derived predicate is registered.
///
/// # Returns
///
/// * `Ok(DerivedPredicate)` - The encoded axiom.
/// * `Err(LirError)` - If the skeleton or the logical expression fails to encode.
///
/// # Errors
///
/// This function returns an error if:
/// * The head of the derived predicate (the formula) is malformed.
/// * The body expression cannot be resolved with the current context.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    ctx: &EncodingContext,
) -> Result<DerivedPredicate, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // 1. Parse the head (the atom being defined)
    // We use the skeleton since this is a definition site
    let head_node_id = node.try_child(0)?;
    let head_node = ast.try_node(head_node_id)?;
    let head = atomic_formula_skeleton::encode(&SyntaxSubtree::new(head_node, head_node_id, ast))?;

    // 2. Parse the body (the logical formula)
    // We use the free expr::encode function to resolve symbols
    let body_node_id = node.try_child(1)?;
    let body_node = ast.try_node(body_node_id)?;
    let body = expr::encode(&SyntaxSubtree::new(body_node, body_node_id, ast), ctx)?;

    Ok(DerivedPredicate::new(head, body))
}
