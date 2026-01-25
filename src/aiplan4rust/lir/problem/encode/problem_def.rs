//! Problem Encoding Orchestration
//!
//! This module iterates through the PDDL problem AST to extract instance-specific
//! information such as objects, the initial state, goal conditions, and HTN
//! initial task networks. It populates the final `LiftedProblem` IR.

use std::collections::HashMap;
use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::encode::{constants_def, expr, goal, init, initial_task_network, EncodingContext};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxSubtree};

/// Encodes a PDDL/HTN problem AST into the Lifted Intermediate Representation (LIR).
///
/// This function performs a pre-order traversal of the problem's syntax tree. It
/// resolves symbols (predicates and functions) using the mapping indices provided
/// from the domain encoding phase.
///
/// # Arguments
///
/// * `context` - The linked semantic context containing the problem's AST and symbol table.
/// * `ir` - The mutable `LiftedProblem` to be populated with initial states, goals, and objects.
/// * `ast_pred_to_idx` - A lookup map to resolve AST predicate references to LIR indices.
/// * `ast_func_to_idx` - A lookup map to resolve AST function references to LIR indices.
///
/// # Returns
///
/// * `Ok(())` - If the problem was successfully encoded.
/// * `Err(LirError)` - If any semantic error (missing symbols) or syntax error is found.
///
/// # Errors
///
/// This function returns an error if:
/// * Symbol resolution fails for the initial state or goal.
/// * The problem name or object definitions are malformed.
/// * A logical expression (metric, constraint, length) fails to encode.
pub fn encode(
    context: &LinkedSemanticContext,
    ir: &mut LiftedProblem,
    ast_pred_to_idx: &mut HashMap<NodeId, usize>,
    ast_func_to_idx: &mut HashMap<NodeId, usize>,
) -> Result<(), LirError> {
    let problem_tree = context.problem_syntax_tree();

    let symbol_table = context.problem_table();
    let ctx = EncodingContext::new(symbol_table, &ast_pred_to_idx, &ast_func_to_idx);

    for (node_id, node) in problem_tree.preorder().ids() {
        let subtree = SyntaxSubtree::new(node, node_id, problem_tree);

        match subtree.node().kind() {
            AstKind::ProblemName => {
                let id = subtree.node().try_ident()?;
                ir.set_problem_id(id)?;
            }
            AstKind::ObjectsDef => {
                ir.add_objects(constants_def::encode(&subtree)?);
            }
            AstKind::Init => {
                let init = init::encode(&subtree, &ctx, ir)?;
                ir.set_init(init);
            }
            AstKind::Goal => {
                let goal_expr = goal::encode(&subtree, &ctx, ir)?;
                ir.set_goal(goal_expr);
            }
            AstKind::Constraints => {
                let constraints = expr::encode(&subtree, &ctx, ir)?;
                ir.set_problem_constraints(constraints);
            }
            AstKind::Metric => {
                let metric =  expr::encode(&subtree, &ctx, ir)?;
                ir.set_metric_spec(metric);
            }
            AstKind::Length => {
                let length =  expr::encode(&subtree, &ctx, ir)?;
                ir.set_length_spec(length);
            }
            AstKind::InitialTaskNetwork => {
                let network = initial_task_network::encode(&subtree, &ctx, ir)?;
                ir.set_initial_task_network(network);
            }
            _ => {}
        }
    }

    Ok(())
}
