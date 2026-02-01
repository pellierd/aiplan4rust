//! Initial Task Network (ITN) Encoding
//!
//! This module handles the entry point of HTN problems by encoding the
//! initial task network, including its optional parameters and subtasks.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::problem::encode::{named_typed_list, task_network, typed_list};
use crate::aiplan4rust::lir::problem::encode::registry::EncodingRegistry;
use crate::aiplan4rust::lir::problem::InitialTaskNetwork;

/// Encodes an `InitialTaskNetwork` from the syntax tree.
///
/// This function parses optional parameters and the mandatory task network body.
/// It acts as the bridge between the problem definition and the HTN decomposition.
///
/// # Arguments
/// * `subtree` - The AST subtree for the initial task network.
/// * `registry` - The registry for resolving task names and types.
/// * `ir` - The mutable lifted problem.
///
/// # Returns
/// * `Ok(InitialTaskNetwork)` - The fully encoded initial state for HTN.
/// * `Err(LirError)` - If parameters are malformed or the task network is invalid.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<InitialTaskNetwork, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    let mut child_index = 0;

    // 1. Parse optional parameters
    let parameters = if let Ok(parameters_def_id) = node.try_child(child_index) {
        let parameters_def_node = ast.try_node(parameters_def_id)?;

        if parameters_def_node.kind() == AstKind::ParametersDef {
            // --- ÉTAPE CRUCIALE : BINDING ---
            // On enregistre les variables dans le registre pour que le Task Network
            // puisse les résoudre (ex: variables existentielles du problème).
            named_typed_list::bind_variables(parameters_def_id, ast, registry)?;

            let param_node_id = parameters_def_node.try_child(0)?;
            let param_node = ast.try_node(param_node_id)?;
            child_index += 1;

            // Encodage de la liste typée
            typed_list::encode(&SyntaxSubtree::new(param_node, param_node_id, ast), registry)?
        } else {
            // Si ce n'est pas un ParametersDef, on s'assure que le registre est vide
            registry.clear_variables();
            TypedList::empty()
        }
    } else {
        registry.clear_variables();
        TypedList::empty()
    };

    // 2. Parse the underlying task network body (subtasks, constraints, etc.)
    let tw_node_id = node.try_child(child_index)?;
    let tw_node = ast.try_node(tw_node_id)?;

    // Le task_network::encode pourra maintenant utiliser registry.try_resolve_variable
    // pour lier les tâches aux paramètres définis ci-dessus.
    let tw = task_network::encode(&SyntaxSubtree::new(tw_node, tw_node_id, ast), registry)?;

    Ok(InitialTaskNetwork::new(parameters, tw))
}
