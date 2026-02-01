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
use crate::aiplan4rust::lir::problem::encode::EncodingRegistry;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
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
    let type_node = subtree.node();
    let symbol_table = registry.symbol_table();
    let mut ty = Type::new();

    // Iterate through children: for a single type, this is one ID;
    // for an 'either' node, it iterates through all member types.
    for primitive_id in type_node.children() {
        // 1. Resolve the usage to its declaration in the symbol table
        let type_declaration = symbol_table.try_resolve_declaration_by_usage(
            *primitive_id,
            SymbolKind::PrimitiveType
        )?;

        // 2. Map the declaration's NodeId to the LIR's TypeID
        let primitive_type_symbol_id = registry.try_resolve_type_symbol(
            type_declaration.node_id()
        )?;

        // 3. Accumulate in the LIR Type structure
        ty.add_type(primitive_type_symbol_id);
    }

    Ok(ty)
}
