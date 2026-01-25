//! Typed List Encoding
//!
//! This module provides functions to parse and encode lists of typed symbols,
//! commonly found in parameters, constants, and object definitions.

use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::problem::encode::typed_symbol;

/// Encodes a `TypedList` from a syntax subtree.
///
/// Iterates through the children of the provided node, encoding each as a
/// `TypedSymbol` and collecting them into a unified list.
///
/// # Arguments
/// * `subtree` - The AST subtree containing a sequence of typed symbols.
///
/// # Returns
/// * `Ok(TypedList)` - A list of successfully encoded typed symbols.
/// * `Err(LirError)` - If any individual symbol fails to encode.
pub fn encode(subtree: &SyntaxSubtree<AstNode>) -> Result<TypedList, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    let mut typed_list = TypedList::new();

    for &id in node.children() {
        let child_node = ast.try_node(id)?;
        let child_subtree = SyntaxSubtree::new(child_node, id, ast);

        // Delegate encoding of each individual symbol to the symbols module
        let symbol = typed_symbol::encode(&child_subtree)?;
        typed_list.push(symbol);
    }

    Ok(typed_list)
}
