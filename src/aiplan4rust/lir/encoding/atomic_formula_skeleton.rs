//! Atomic Formula Skeleton Encoding
//!
//! This module handles the encoding of atomic formula structures (predicates).
//! It specializes the generic `NamedTypedList` into an `AtomicFormulaSkeleton`,
//! representing the declaration of a predicate and its parameter signature.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::PredicateSymbolId;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::encoding::{typed_list, EncodingRegistry};
use crate::aiplan4rust::lir::problem::atomic_skeleton::{AtomicFormulaSkeleton};

/// Encodes an atomic formula skeleton from the syntax tree.
///
/// This function leverages the generic `named_typed_list` encoder to extract
/// the predicate symbol and its typed parameters, then wraps them into an
/// `AtomicFormulaSkeleton`.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the predicate (e.g., `(at ?r - robot ?l - location)`).
/// * `evaluator` - The symbol evaluator for either_type resolution.
///
/// # Returns
///
/// * `Ok(AtomicFormulaSkeleton)` - The encoded predicate signature.
/// * `Err(LirError)` - If the name or the parameter list is malformed.
///
/// # Errors
///
/// Returns an error if the underlying `named_typed_list::encoding` fails,
/// typically due to a missing identifier or an unknown either_type.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    predicate_id: PredicateSymbolId,
) -> Result<AtomicFormulaSkeleton, LirError> {
    registry.clear_variables();

    let node = subtree.node();
    let ast = subtree.tree();

    // On encode directement les paramètres (second enfant)
    let params_node_id = node.try_child(1)?;
    let params_node = ast.try_node(params_node_id)?;
    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(params_node, params_node_id, ast),
        registry
    )?;

    // On construit avec l'ID déjà fourni
    let variable_symbols = registry.get_variable_symbols();
    let formula = AtomicFormulaSkeleton::new(
        predicate_id,
        parameters
    )
        .with_variable_symbols(variable_symbols);
    Ok(formula)

}
