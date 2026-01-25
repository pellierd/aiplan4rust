use std::collections::HashSet;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::typed_symbol;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;

/// Extracts types from a `TypesDef` syntax subtree into a set of TypedSymbols.
pub fn endode(subtree: &SyntaxSubtree<AstNode>) -> Result<HashSet<TypedSymbol>, LirError> {
    // 1. On récupère le premier enfant (le conteneur de la liste typée)
    let list_node_id = subtree.node().try_child(0)?;
    let list_node = subtree.tree().try_node(list_node_id)?;

    // 2. On itère sur ses enfants pour transformer chaque élément en TypedSymbol
    let mut types = HashSet::new();
    for &child_id in list_node.children() {
        let child_node = subtree.tree().try_node(child_id)?;
        let child_subtree = SyntaxSubtree::new(child_node, child_id, subtree.tree());

        // On utilise TryFrom pour convertir le subtree en TypedSymbol
        types.insert(typed_symbol::encode(&child_subtree)?);
    }

    Ok(types)
}
