//! PDDL Type Encoding
//!
//! This module handles the semantic resolution of PDDL types from the AST into the LIR.
//! It supports atomic types and compound types (using the `either` construct).
//!
//! The encoding process follows a strict two-pass architecture:
//! 1. **Phase 1 (Collection)**: All typing identifiers are collected and assigned a [`TypeId`].
//! 2. **Phase 2 (Resolution)**: This module resolves the actual typing references and
//!    inheritance hierarchies using the pre-populated [`EncodingRegistry`].

use crate::aiplan4rust::lang::{Type, TypeId};
use crate::aiplan4rust::lir::old::encoding::EncodingRegistry;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encodes a PDDL Type from the syntax tree by resolving its identifiers.
///
/// This function transforms a typing-related AST node into a resolved [`Type<TypeId>`].
/// It is responsible for:
/// - Resolving single parent types (e.g., in `:types` declarations).
/// - Resolving complex types in typed lists (e.g., `?obj - (either type1 type2)`).
/// - Linking AST symbols to their internal LIR [`TypeId`] via the evaluator.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the typing definition.
/// * `evaluator` - The encoding evaluator containing the resolved symbol-to-typing mapping.
///
/// # Returns
///
/// * `Ok(Type<TypeID>)` - A resolved LIR typing object.
/// * `Err(LirError)` - If a typing symbol is unknown or the AST structure is malformed.
///
/// # Errors
///
/// Returns a [`LirError`] if:
/// - A symbol is encountered that was not registered during Phase 1.
/// - The resolution against the `symbol_table` fails for the `PrimitiveType` kind.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &EncodingRegistry,
) -> Result<Type<TypeId>, LirError> {
    let mut ty = Type::new();
    let tree = subtree.tree();

    for primitive_id in subtree.node().children() {
        // TENTATIVE 1 : Par NodeId (très rapide)
        if let Some(type_id) = registry.resolve_type_symbol(*primitive_id) {
            ty.add_type(type_id);
        } else {
            // TENTATIVE 2 : Fallback par StringId
            // On extrait le nom du nœud actuel (ex: l'ID de "object")
            let name_id = tree.try_node(*primitive_id)?.try_ident()?;

            if let Some(type_id) = registry.resolve_type_symbol_by_name(name_id) {
                ty.add_type(type_id);
            } else {
                // Si même par nom on ne trouve rien, le typing n'existe vraiment pas
                return Err(
                    SymbolTableError::declaration_not_found_for_usage(*primitive_id).into(),
                );
            }
        }
    }

    Ok(ty)
}
