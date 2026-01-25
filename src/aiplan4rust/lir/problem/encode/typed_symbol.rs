//! Typed Symbol Encoding
//!
//! This module provides utilities to encode PDDL symbols (constants, objects, or parameters)
//! and bind them to their respective types during the LIR translation.

use crate::aiplan4rust::lang::{TypedSymbol, Type};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxSubtree};
use crate::aiplan4rust::lir::problem::encode::ty;

/// Encodes a TypedSymbol (a name associated with a Type) from the syntax tree.
///
/// This is used to encode individual constants, objects, or parameters where
/// a name is optionally followed by a type definition.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the symbol and its optional type.
///
/// # Returns
///
/// * `Ok(TypedSymbol)` - The encoded symbol with its resolved type.
/// * `Err(LirError)` - If the identifier is missing or the type encoding fails.
///
/// # Errors
///
/// This function returns an error if:
/// * The first child (the symbol name) cannot be converted to an identifier.
/// * The second child (the type), if present, fails to encode.
pub fn encode(subtree: &SyntaxSubtree<AstNode>) -> Result<TypedSymbol, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();
    let children = node.children();

    // The first child is always the symbol/identifier
    let symbol_node = ast.try_node(children[0])?;
    let name = symbol_node.try_ident()?;

    // The second child is the optional type definition
    let ty = if children.len() > 1 {
        let ty_node_id = children[1];
        let ty_node = ast.try_node(ty_node_id)?;
        ty::encode(&SyntaxSubtree::new(ty_node, ty_node_id, ast))?
    } else {
        Type::new() // Default to untyped/object
    };

    Ok(TypedSymbol::new(name, ty))
}
