//! Global Symbol Encoding
//!
//! This module handles the extraction of constants (from domain) and objects
//! (from problem). It ensures that all symbols are parsed with their
//! respective types and stored in a collection for global resolution.

use std::collections::HashSet;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::typed_symbol;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;

/// Encodes a collection of constants or objects from a syntax subtree.
///
/// This function parses a list of typed symbols (e.g., `:constants` or `:objects` blocks)
/// and returns them as a `HashSet`. This ensures that each symbol name is unique
/// within the global scope.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the constant or object definition block.
///
/// # Returns
///
/// * `Ok(HashSet<TypedSymbol>)` - A set of all uniquely identified typed symbols.
/// * `Err(LirError)` - If the list structure is malformed or a symbol cannot be parsed.
///
/// # Errors
///
/// This function returns an error if:
/// * The child node containing the actual list of symbols cannot be accessed.
pub(crate) fn encode(subtree: &SyntaxSubtree<AstNode>) -> Result<HashSet<TypedSymbol>, LirError> {
    // 1. Accès au premier enfant (le conteneur de la liste d'objets)
    let list_node_id = subtree.node().try_child(0)?;
    let list_node = subtree.tree().try_node(list_node_id)?;

    // 2. Transformation directe des enfants en TypedSymbol
    let mut constants = HashSet::new();
    for &child_id in list_node.children() {
        let child_node = subtree.tree().try_node(child_id)?;
        let child_subtree = SyntaxSubtree::new(child_node, child_id, subtree.tree());

        constants.insert(typed_symbol::encode(&child_subtree)?);
    }

    Ok(constants)
}
