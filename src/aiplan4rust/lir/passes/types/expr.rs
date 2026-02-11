use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeID};
use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::tree::NodeId;

/// Flattens all union types (`Type::Either`) within an expression tree in place.
///
/// This function traverses the entire expression tree (preconditions, effects, constraints, etc.)
/// and replaces any union types with their corresponding flattened primitive identifiers.
/// It specifically targets variable declarations within quantifiers (`forall`, `exists`).
///
/// # Parameters
/// - `expr`: The expression tree to modify.
/// - `map`: A mapping from union types to their unique flattened primitive `TypeID` (pivots).
///
/// # Returns
/// - `Ok(())` if the expression tree was successfully traversed and updated.
/// - `Err(LirError)` if the root node or any child node is inaccessible.
pub fn flatten(expr: &mut Expr, map: &HashMap<Type<TypeID>, TypeID>) -> Result<(), LirError> {
    if expr.is_empty() {
        return Ok(());
    }
    let root_id = expr.try_root_id()?;
    flatten_from_node(expr, root_id, map)
}

/// Internal DFS traversal that processes the expression tree starting from `node_id`.
///
/// This function handles the recursive nature of expressions by using a stack-based
/// depth-first search to avoid stack overflow on deep trees.
///
/// # Specific Logic
/// For nodes of kind `ExprKind::Forall` or `ExprKind::Exists`, the function updates
/// the type of each bound variable to ensure type consistency during grounding.
fn flatten_from_node(expr: &mut Expr, node_id: NodeId, map: &HashMap<Type<TypeID>, TypeID>) -> Result<(), LirError> {
    let mut stack = vec![node_id];

    while let Some(current_id) = stack.pop() {
        let node = expr.try_node_mut(current_id)?;

        // Case: Quantifiers (Variable scope declaration)
        // We must update the types of the variables introduced here.
        if matches!(node.kind(), ExprKind::Forall | ExprKind::Exists) {
            let vars = node.content_mut().try_quantifier_vars_mut()?;
            for var in vars.iter_mut() {
                // If the variable type is a union present in the map, we pivot it.
                if let Some(&new_id) = map.get(var.ty()) {
                    var.set_ty(Type::primitive(new_id));
                }
            }
        }

        // Add children to the stack to continue the DFS traversal
        // Note: We use child IDs to avoid borrowing conflicts with the node mutability.
        for &child_id in node.children() {
            stack.push(child_id);
        }
    }

    Ok(())
}
