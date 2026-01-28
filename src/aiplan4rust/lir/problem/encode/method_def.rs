use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::syntax::tree::SyntaxSubtree;
use crate::aiplan4rust::lir::problem::encode::{expr, named_typed_list, task_network};
use crate::aiplan4rust::lir::problem::encode::registry::EncodingContext;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::problem::method::Method;

/// Encodes an HTN method from the syntax tree into the LIR.
///
/// Processes the method's signature, the abstract task it implements,
/// its optional preconditions, and the resulting task network decomposition.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the method definition.
/// * `ctx` - The encoding context for symbol and parameter resolution.
/// * `ir` - The mutable `LiftedProblem` where the method is registered.
///
/// # Returns
///
/// * `Ok(Method)` - The encoded HTN method.
/// * `Err(LirError)` - If the structure is invalid or any component fails to encode.
///
/// # Errors
///
/// This function returns an error if:
/// * The method header or the implemented task expression is malformed.
/// * The mandatory task network definition is missing.
/// * Symbol resolution fails within the precondition or the task network.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    ctx: &EncodingContext,
) -> Result<Method, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // 1. Parse header (name + parameters) using the free function
    let header = named_typed_list::encode(subtree)?;

    // 2. Access the method body
    let def_body_node = ast.try_node(node.try_child(2)?)?;
    let children = def_body_node.children();
    let mut child_index = 0;

    // 3. Parse the abstract task expression this method achieves
    let task_node_id = children[child_index];
    let task_node = ast.try_node(task_node_id)?;
    let task = expr::encode(&SyntaxSubtree::new(task_node, task_node_id, ast), ctx)?;
    child_index += 1;

    // 4. Parse optional precondition
    let precondition = if children.len() > child_index {
        let pre_node_def = ast.try_node(children[child_index])?;
        if pre_node_def.kind() == AstKind::MethodPreconditionDef {
            let pre_node_id = pre_node_def.try_child(0)?;
            let pre_node = ast.try_node(pre_node_id)?;
            child_index += 1;
            expr::encode(&SyntaxSubtree::new(pre_node, pre_node_id, ast), ctx)?
        } else {
            Expr::empty_or()
        }
    } else {
        Expr::empty_or()
    };

    // 5. Parse the decomposition task network
    // Note: We expect the task network to be the next child
    let tw_node_def_id  = children[child_index];
    let tw_node_def = ast.try_node(tw_node_def_id)?;
    let task_network = task_network::encode(&SyntaxSubtree::new(tw_node_def, tw_node_def_id, ast), ctx)?;

    Ok(Method::from_header(header, task, precondition, task_network))
}
