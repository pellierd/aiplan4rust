use crate::aiplan4rust::lang::{Type, TypeId};
use crate::aiplan4rust::lir::encoding::registry::EncodingRegistry;
use crate::aiplan4rust::lir::encoding::EncodingError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::syntax::ast::tree::SyntaxSubtree;
use crate::aiplan4rust::syntax::ast::AstNode;

/// Encodes a PDDL Type from the syntax tree by resolving its identifiers.
///
/// This function supports both single types and compound types (either).
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &EncodingRegistry,
) -> Result<Type<TypeId>, EncodingError> {
    let mut ty = Type::new();
    let tree = subtree.tree();

    for &primitive_id in subtree.node().children() {
        // Étape 1 : Résolution directe par NodeId (vissage O(1))
        if let Some(type_id) = registry.resolve_type_symbol(primitive_id) {
            ty.add_type(type_id);
            continue;
        }

        // Étape 2 : Fallback par identifiant (nom) pour les types built-in/globaux
        let symbol_id = tree.try_node(primitive_id)?.try_ident()?;
        if let Some(type_id) = registry.resolve_type_symbol_by_name(symbol_id) {
            ty.add_type(type_id);
        } else {
            // Échec total : on utilise SymbolTableError, encapsulé dans EncodingError
            return Err(SymbolTableError::declaration_not_found_for_usage(primitive_id).into());
        }
    }

    Ok(ty)
}
