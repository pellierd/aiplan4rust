//! Typed Symbol Encoding
//!
//! This module provides utilities to encode PDDL symbols (such as constants, objects,
//! or parameters) and bind them to their respective types during LIR translation.
//!
//! It processes nodes that pair identifiers with their type definitions, ensuring
//! that types are resolved against the provided `EncodingContext`.

use crate::aiplan4rust::lang::{TypedSymbol, Type, TypeID};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{Node, SyntaxSubtree};
use crate::aiplan4rust::lir::problem::encode::{ty, EncodingRegistry};

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
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &EncodingRegistry,
) -> Result<TypedSymbol<TypeID>, LirError> {
    let typed_symbol_node = subtree.node();
    let ast = subtree.tree();
    let children = typed_symbol_node.children();

    // The first child is the symbol/identifier (e.g., the 'x' in 'x - type1')
    let symbol_node = ast.try_node(children[0])?;
    let symbol_id = symbol_node.try_ident()?;

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

    Ok(TypedSymbol::new(symbol_id, ty))
}
