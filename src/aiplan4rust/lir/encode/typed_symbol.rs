//! Typed Symbol Encoding
//!
//! This module provides utilities to encode PDDL symbols (such as constants, objects,
//! or parameters) and bind them to their respective types during LIR translation.
//!
//! It processes nodes that pair identifiers with their type definitions, ensuring
//! that types are resolved against the provided `EncodingContext`.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{TypedSymbol, Type, TypeID, VariableID, ObjectID};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::encode::{ty, EncodingRegistry};

/// Encodes a `TypedSymbol` (an identifier associated with a Type) from the syntax tree.
///
/// This function extracts a symbol name and resolves its associated type(s). It is
/// commonly used for parsing action parameters, constants in a domain, or objects
/// in a problem definition.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the symbol and its optional type.
/// * `registry` - The encoding context used to resolve type identifiers.
///
/// # Returns
///
/// * `Ok(TypedSymbol<TypeID>)` - The encoded symbol with its resolved numeric TypeIDs.
/// * `Err(LirError)` - If the identifier is missing or type resolution fails.
///
/// # Errors
///
/// This function will return an error if:
/// * The first child (the symbol name) cannot be resolved to a valid identifier.
/// * The second child (the type definition), if present, fails the encoding process
///   (e.g., refers to an unregistered type).
pub fn encode_typed_type(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &EncodingRegistry,
) -> Result<TypedSymbol<TypeID, TypeID>, LirError> {
    let typed_symbol_node = subtree.node();
    let ast = subtree.tree();
    let children = typed_symbol_node.children();

    // The first child is the symbol/identifier (e.g., the 'x' in 'x - type1')
    let symbol_node = ast.try_node(children[0])?;
    let symbol_id = symbol_node.try_ident()?;

    let type_id = registry.try_resolve_type_symbol_by_name(symbol_id)?;

    // The second child contains the type definitions (e.g., 'type1' or an 'either' block)
    let ty = if children.len() > 1 {
        let ty_node_id = children[1];
        let ty_node = ast.try_node(ty_node_id)?;
        // Delegate to the type encoding module to resolve the TypeID(s)
        ty::encode(&SyntaxSubtree::new(ty_node, ty_node_id, ast), registry)?
    } else {
        // Fallback to an empty Type (representing 'object' or untyped) if no type is provided
        Type::new()
    };

    Ok(TypedSymbol::new(type_id, ty))
}

pub fn encode_typed_object(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &EncodingRegistry,
) -> Result<TypedSymbol<ObjectID, TypeID>, LirError> {
    let typed_symbol_node = subtree.node();
    let ast = subtree.tree();

    // 1. Récupération du StringID (le usize)
    let symbol_node_id = typed_symbol_node.try_child(0)?;
    let symbol_node = ast.try_node(symbol_node_id)?;
    let symbol_id = symbol_node.try_ident()?; // Récupère le StringID (usize)

    // 2. Résolution globale par StringID
    let object_id = registry.try_resolve_object_symbol_by_name(symbol_id)?;

    // 3. Encodage du type (à droite du tiret)
    let ty = if typed_symbol_node.children().len() > 1 {
        let ty_node_id = typed_symbol_node.children()[1];
        let ty_node = ast.try_node(ty_node_id)?;
        ty::encode(&SyntaxSubtree::new(ty_node, ty_node_id, ast), registry)?
    } else {
        Type::new()
    };

    Ok(TypedSymbol::new(object_id, ty))
}
pub fn encode_typed_variable(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<TypedSymbol<VariableID, TypeID>, LirError> {
    let typed_symbol_node = subtree.node();
    let ast = subtree.tree();
    let children = typed_symbol_node.children();

    let variable_id = registry.register_variable(children[0]);

    // The second child contains the type definitions (e.g., 'type1' or an 'either' block)
    let ty = if children.len() > 1 {
        let ty_node_id = children[1];
        let ty_node = ast.try_node(ty_node_id)?;
        // Delegate to the type encoding module to resolve the TypeID(s)
        ty::encode(&SyntaxSubtree::new(ty_node, ty_node_id, ast), registry)?
    } else {
        // Fallback to an empty Type (representing 'object' or untyped) if no type is provided
        Type::new()
    };

    Ok(TypedSymbol::new(variable_id, ty))
}
