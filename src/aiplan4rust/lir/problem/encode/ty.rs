//! PDDL Type Encoding
//!
//! This module provides functions to extract and represent PDDL types from the AST.
//! It supports both single types and compound types (e.g., `either` definitions).

use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::{SyntaxContent, SyntaxNode, SyntaxSubtree};

/// Encodes a PDDL Type from the syntax tree.
///
/// This function extracts one or more type identifiers from a type definition node.
/// It is typically used when parsing typed lists (e.g., `?x - type1`) or function
/// return types in the domain and problem files.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the type definition (e.g., an `Either` node or a single type).
///
/// # Returns
///
/// * `Ok(Type)` - A `Type` object containing all collected type identifiers.
/// * `Err(LirError)` - If a child node fails to provide a valid identifier.
///
/// # Errors
///
/// This function will return an error if:
/// * A child node expected to be an identifier is missing content.
/// * The AST structure does not allow identifier extraction via `try_ident()`.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<Type, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    let mut ty = Type::new();

    for ty_id in node.children() {
        let child_node = ast.try_node(*ty_id)?;

        // Ensure the node has content before attempting to extract an identifier
        if !child_node.content().is_none() {
            ty.add_type(child_node.try_ident()?);
        }
    }

    Ok(ty)
}
