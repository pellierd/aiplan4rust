//! Derived Predicate Encoding
//!
//! This module handles the encoding of PDDL derived predicates (axioms).
//! Derived predicates allow the domain to define new relations based on
//! existing ones, which are automatically updated as the state changes.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::derived_predicate::DerivedPredicate;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::encode::{atomic_formula_skeleton, expr, named_typed_list};
use crate::aiplan4rust::lir::encode::registry::EncodingRegistry;
use crate::aiplan4rust::semantic::symbol::SymbolKind;

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

    // 1. The Head is at index 0 (e.g., (reachable ?x ?y))
    let head_node_id = node.try_child(0)?;
    let head_node = ast.try_node(head_node_id)?;

    // --- ID Resolution ---
    // The first child of the head_node is the predicate identifier (the symbol name).
    let predicate_symbol_id = head_node.children()[0];

    // Resolve the declaration in the symbol table.
    let declaration = registry.symbol_table()
        .try_resolve_declaration_by_usage(predicate_symbol_id, SymbolKind::Predicate)?;

    // Retrieve the unique AtomSkeletonID for this predicate.
    let head_id = registry.try_resolve_atom_skeleton(declaration.node_id())?;

    // Clear local variables before binding the head parameters.
    registry.clear_variables();

    // 2. Encode the Head skeleton.
    // This binds the parameters (e.g., ?x, ?y) in the registry for the body to use.
    let head = atomic_formula_skeleton::encode(
        &SyntaxSubtree::new(head_node, head_node_id, ast),
        registry
    )?;

    // 3. Encode the Body (the logical expression is at index 1).
    let body_node_id = node.try_child(1)?;
    let body_node = ast.try_node(body_node_id)?;

    // expr::encode can now resolve variables from the head since they are in the registry.
    let body = expr::encode(
        &SyntaxSubtree::new(body_node, body_node_id, ast),
        registry
    )?;

    Ok(DerivedPredicate::new(head_id, head, body))
}
