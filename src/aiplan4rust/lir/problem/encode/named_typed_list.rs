//! Named Typed List Encoding
//!
//! This module provides a generic encoder for constructs that bind a name to a
//! signature (a list of typed parameters).
//!
//! It acts as the "skeleton" builder for various PDDL and HTN components,
//! including:
//! - **Predicates** and **Functions** (Domain declarations)
//! - **Actions** and **Durative Actions** (Operator signatures)
//! - **Tasks** and **Methods** (HTN definitions)
//!
//! By centralizing this logic, the LIR ensures consistent handling of identifier
//! extraction and parameter scope across all operators.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::{NodeId, Node, SyntaxSubtree, Tree};
use crate::aiplan4rust::lir::problem::encode::{typed_list, EncodingRegistry};

/// Encodes a `NamedTypedList` (skeleton) from a syntax subtree.
///
/// This function extracts the mandatory operator name (first child) and processes
/// its parameter signature. It is designed to be flexible, handling parameters
/// whether they are wrapped in an explicit `ParametersDef` node (common in actions)
/// or provided as a direct list (common in predicates).
///
/// # Arguments
///
/// * `subtree` - The AST subtree representing the named declaration.
/// * `registry` - The symbol registry for type resolution.
///
/// # Returns
///
/// * `Ok(NamedTypedList)` - A skeleton containing the `StringID` of the name and the `TypedList`.
/// * `Err(LirError)` - If the name is missing, not an identifier, or if parameters are malformed.
///
/// # Errors
///
/// This function returns an error if:
/// * The node has no children (missing name).
/// * The first child is not a valid identifier.
/// * The parameter encoding fails due to unknown types in the `EncodingRegistry`.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &EncodingRegistry
) -> Result<NamedTypedList, LirError> {
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
        // If parameters are explicitly wrapped in a definition block (e.g., :parameters (?a - type))
        AstKind::ParametersDef => {
            let parameters_id = second_child_node.try_child(0)?;
            let parameters_node = ast.try_node(parameters_id)?;
            typed_list::encode(&SyntaxSubtree::new(parameters_node, parameters_id, ast), registry)?
        }
        // If the parameters are directly under the node (e.g., (at ?l - location))
        _ => {
            typed_list::encode(&SyntaxSubtree::new(second_child_node, second_child_id, ast), registry)?
        }
    };

    Ok(NamedTypedList::new(name, parameters))
}

/// Helper universel pour lier les variables d'une liste de paramètres au registre.
///
/// Cette fonction gère le cas où les variables sont enveloppées dans un ParameterDef
/// ou présentes directement dans la liste.
pub fn bind_variables(
    params_root_id: NodeId,
    ast: &Tree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<(), LirError> {
    registry.clear_variables();

    let root_node = ast.try_node(params_root_id)?;

    // Si on pointe sur un ParameterDef (ex: Action), on descend d'un cran.
    // Sinon (ex: Task), on utilise le nœud directement.
    let target_node = if root_node.kind() == AstKind::ParametersDef {
        let content_id = root_node.try_child(0)?;
        ast.try_node(content_id)?
    } else {
        root_node
    };

    for &child_id in target_node.children() {
        let child_node = ast.try_node(child_id)?;
        if child_node.kind() == AstKind::Variable {
            registry.register_variable(child_id);
        }
    }

    Ok(())
}
