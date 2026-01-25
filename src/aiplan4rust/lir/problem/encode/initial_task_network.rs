//! Initial Task Network (ITN) Encoding
//!
//! This module handles the entry point of HTN problems by encoding the
//! initial task network, including its optional parameters and subtasks.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::TypedList;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::problem::encode::{task_network, typed_list};
use crate::aiplan4rust::lir::problem::encode::context::EncodingContext;
use crate::aiplan4rust::lir::problem::{InitialTaskNetwork, LiftedProblem};

/// Encodes an `InitialTaskNetwork` from the syntax tree.
///
/// This function parses optional parameters and the mandatory task network body.
/// It acts as the bridge between the problem definition and the HTN decomposition.
///
/// # Arguments
/// * `subtree` - The AST subtree for the initial task network.
/// * `ctx` - The encoding context for resolving task names and types.
/// * `ir` - The mutable lifted problem.
///
/// # Returns
/// * `Ok(InitialTaskNetwork)` - The fully encoded initial state for HTN.
/// * `Err(LirError)` - If parameters are malformed or the task network is invalid.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    ctx: &EncodingContext,
    ir: &mut LiftedProblem,
) -> Result<InitialTaskNetwork, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    let mut child_index = 0;

    // 1. Parse optional parameters
    let parameters = if let Ok(parameters_def_id) = node.try_child(child_index) {
        let parameters_def_node = ast.try_node(parameters_def_id)?;
        if parameters_def_node.kind() == AstKind::ParametersDef {
            let param_node_id = parameters_def_node.try_child(0)?;
            let param_node = ast.try_node(param_node_id)?;
            child_index += 1;
            // Use the new free function for typed lists
            typed_list::encode(&SyntaxSubtree::new(param_node, param_node_id, ast))?
        } else {
            TypedList::empty()
        }
    } else {
        TypedList::empty()
    };

    // 2. Parse the underlying task network body
    let tw_node_id = node.try_child(child_index)?;
    let tw_node = ast.try_node(tw_node_id)?;

    // Delegate to the specialized task network encoder
    let tw = task_network::encode(&SyntaxSubtree::new(tw_node, tw_node_id, ast), ctx, ir)?;

    Ok(InitialTaskNetwork::new(parameters, tw))
}
