//! Named Typed List Encoding
//!
//! This module handles the encoding of structures that associate a name with
//! a list of typed parameters. It is commonly used for predicate declarations,
//! function signatures, or task definitions in PDDL/HTN.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxSubtree};
use crate::aiplan4rust::lir::problem::encode::typed_list;

/// Encodes a `NamedTypedList` from a syntax subtree.
///
/// This function extracts a mandatory identifier (the name) and its associated
/// parameters. It automatically handles cases where parameters are wrapped
/// in a `ParametersDef` node or provided as a raw list.
///
/// # Arguments
/// * `subtree` - The AST subtree representing the named list (e.g., a predicate signature).
///
/// # Returns
/// * `Ok(NamedTypedList)` - The successfully encoded name and parameter list.
/// * `Err(LirError)` - If the name is missing or the parameter list fails to encode.
///
/// # Errors
/// This function returns an error if:
/// * The first child is missing or is not a valid identifier.
/// * The parameter definition is malformed.
pub fn encode(subtree: &SyntaxSubtree<AstNode>) -> Result<NamedTypedList, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // 1. Extract the mandatory name (first child)
    let name_id = node.try_child(0)?;
    let name_node = ast.try_node(name_id)?;
    let name = name_node.try_ident()?;

    // 2. Extract the parameters (second child)
    let second_child_id = node.try_child(1)?;
    let second_child_node = ast.try_node(second_child_id)?;

    let parameters = match second_child_node.kind() {
        // If parameters are explicitly wrapped in a definition block
        AstKind::ParametersDef => {
            let parameters_id = second_child_node.try_child(0)?;
            let parameters_node = ast.try_node(parameters_id)?;
            typed_list::encode(&SyntaxSubtree::new(parameters_node, parameters_id, ast))?
        }
        // If the parameters are directly under the node
        _ => {
            typed_list::encode(&SyntaxSubtree::new(second_child_node, second_child_id, ast))?
        }
    };

    Ok(NamedTypedList::new(name, parameters))
}
