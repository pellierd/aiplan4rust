//! PDDL Type Encoding
//!
//! This module handles the semantic resolution of PDDL types from the AST into the LIR.
//! It supports atomic types and compound types (using the `either` construct).
//!
//! The encoding process follows a strict two-pass architecture:
//! 1. **Phase 1 (Collection)**: All type identifiers are collected and assigned a [`TypeID`].
//! 2. **Phase 2 (Resolution)**: This module resolves the actual type references and
//!    inheritance hierarchies using the pre-populated [`EncodingRegistry`].

use crate::aiplan4rust::lang::{Type, TypeID};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::LirError::SymbolTable;
use crate::aiplan4rust::lir::problem::encode::EncodingRegistry;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encodes a PDDL Type from the syntax tree by resolving its identifiers.
///
/// This function transforms a type-related AST node into a resolved [`Type<TypeID>`].
/// It is responsible for:
/// - Resolving single parent types (e.g., in `:types` declarations).
/// - Resolving complex types in typed lists (e.g., `?obj - (either type1 type2)`).
/// - Linking AST symbols to their internal LIR [`TypeID`] via the registry.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the type definition.
/// * `registry` - The encoding registry containing the resolved symbol-to-type mapping.
///
/// # Returns
///
/// * `Ok(Type<TypeID>)` - A resolved LIR type object.
/// * `Err(LirError)` - If a type symbol is unknown or the AST structure is malformed.
///
/// # Errors
///
/// Returns a [`LirError`] if:
/// - A symbol is encountered that was not registered during Phase 1.
/// - The resolution against the `symbol_table` fails for the `PrimitiveType` kind.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &EncodingRegistry,
) -> Result<Type<TypeID>, LirError> {
    let mut ty = Type::new();
    let tree = subtree.tree();

    for primitive_id in subtree.node().children() {
        // TENTATIVE 1 : Par NodeId (très rapide)
        if let Some(type_id) = registry.resolve_type_symbol(*primitive_id) {
            ty.add_type(type_id);
        }
        else {
            // TENTATIVE 2 : Fallback par StringId
            // On extrait le nom du nœud actuel (ex: l'ID de "object")
            let name_id = tree.try_node(*primitive_id)?.try_ident()?;

            if let Some(type_id) = registry.resolve_type_by_name(name_id) {
                ty.add_type(type_id);
            } else {
                // Si même par nom on ne trouve rien, le type n'existe vraiment pas
                return Err(SymbolTableError::declaration_not_found_for_usage(*primitive_id).into());
            }
        }
    }

    Ok(ty)
}
