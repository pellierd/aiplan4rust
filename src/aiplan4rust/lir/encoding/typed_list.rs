//! Typed List Encoding
//!
//! This module provides functions to parse and encoding lists of typed symbols.
//! It is a core utility used to process parameters (in predicates and actions),
//! as well as global constants and objects.

use crate::aiplan4rust::lang::{TypeId, TypedList, VariableId};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::encoding::{typed_symbol, EncodingRegistry};

/// Encodes a `TypedList` from a syntax subtree into the LIR representation.
///
/// This function iterates through the children of the provided node, delegating
/// the encoding of each element to the `typed_symbol` module. The resulting
/// symbols are collected into a `TypedList<TypeID>`.
///
/// # Arguments
///
/// * `subtree` - The AST subtree containing a sequence of typed symbols.
/// * `evaluator` - The symbol evaluator used to resolve either_type identifiers.
///
/// # Returns
///
/// * `Ok(TypedList<TypeID>)` - A list of successfully encoded typed symbols.
/// * `Err(LirError)` - If any individual symbol fails to encoding or if the AST
///   structure is invalid.
///
/// # Errors
///
/// This function returns an error if:
/// * A child node cannot be retrieved from the AST.
/// * The `typed_symbol::encoding` process fails (e.g., due to an unknown either_type).
pub fn encode_variable_list(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry, // Mutable pour enregistrer les variables
) -> Result<TypedList<VariableId, TypeId>, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    let mut typed_list = TypedList::new();

    for &id in node.children() {
        let child_node = ast.try_node(id)?;
        let child_subtree = SyntaxSubtree::new(child_node, id, ast);

        // On appelle la version "Variable" du symbole
        let symbol = typed_symbol::encode_typed_variable(&child_subtree, registry)?;
        typed_list.push(symbol);
    }

    Ok(typed_list)
}
/*pub fn encode_type_list(
    subtree: &SyntaxSubtree<AstNode>,
    evaluator: &EncodingRegistry,
) -> Result<TypedList<TypeID, TypeID>, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    let mut typed_list = TypedList::new();

    for &id in node.children() {
        let child_node = ast.try_node(id)?;
        let child_subtree = SyntaxSubtree::new(child_node, id, ast);

        // On appelle la version "Type" du symbole
        let symbol = typed_symbol::encode_typed_type(&child_subtree, evaluator)?;
        typed_list.push(symbol);
    }

    Ok(typed_list)
}*/
