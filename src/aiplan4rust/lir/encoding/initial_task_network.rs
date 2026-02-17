//! Initial Task Network (ITN) Encoding
//!
//! This module handles the entry point of HTN problems by encoding the
//! initial task network, including its optional parameters and subtasks.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lir::{InitialTaskNetwork, LirError};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::encoding::{task_network, typed_list};
use crate::aiplan4rust::lir::encoding::registry::EncodingRegistry;

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

    registry.clear_variables();
    registry.clear_task_labels();


    let mut child_index = 0;

    // 1. Parse optional parameters
    let parameters = if let Ok(parameters_def_id) = node.try_child(child_index) {
        let parameters_def_node = ast.try_node(parameters_def_id)?;

        if parameters_def_node.kind() == AstKind::ParametersDef {
            let param_node_id = parameters_def_node.try_child(0)?;
            let param_node = ast.try_node(param_node_id)?;
            child_index += 1;

            // Encodage de la liste typée
            typed_list::encode_variable_list(&SyntaxSubtree::new(param_node, param_node_id, ast), registry)?
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

    // Le task_network::encoding pourra maintenant utiliser registry.try_resolve_variable
    // pour lier les tâches aux paramètres définis ci-dessus.
    let tw = task_network::encode(&SyntaxSubtree::new(tw_node, tw_node_id, ast), registry)?;
    let variable_symbols = registry.get_variable_symbols();
    let task_label_symbols = registry.get_task_label_symbols();
    let init_tw = InitialTaskNetwork::new(
        parameters,
        tw
    )
        .with_variable_symbols(variable_symbols)
        .with_task_label_symbols(task_label_symbols);
    Ok(init_tw)
}
