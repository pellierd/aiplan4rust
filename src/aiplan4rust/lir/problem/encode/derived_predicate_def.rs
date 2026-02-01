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
use crate::aiplan4rust::lir::problem::encode::{atomic_formula_skeleton, expr, named_typed_list};
use crate::aiplan4rust::lir::problem::encode::registry::EncodingRegistry;

/// Encodes a derived predicate from the syntax tree into the LIR.
///
/// This function translates a PDDL axiom by encoding its "head" (the predicate
/// being defined) and its "body" (the logical condition that makes it true).
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `:derived` definition.
/// * `registry` - The registry for resolving identifiers within the logic.
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
    registry: &mut EncodingRegistry,
) -> Result<DerivedPredicate, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // 1. Le Head est à l'index 0 (ex: (ontable ?x))
    let head_node_id = node.try_child(0)?;
    let head_node = ast.try_node(head_node_id)?;

    // --- ÉTAPE 1 : Binding des variables ---
    named_typed_list::bind_variables(head_node_id, ast, registry)?;

    // 2. Encode le Head (maintenant que les variables sont bindées)
    let head = atomic_formula_skeleton::encode(
        &SyntaxSubtree::new(head_node, head_node_id, ast),
        registry
    )?;

    // 3. Encode le Body (l'index 1 est la formule logique)
    let body_node_id = node.try_child(1)?;
    let body_node = ast.try_node(body_node_id)?;

    // expr::encode pourra résoudre les variables du head car elles sont dans le registry
    let body = expr::encode(
        &SyntaxSubtree::new(body_node, body_node_id, ast),
        registry
    )?;

    Ok(DerivedPredicate::new(head, body))
}
