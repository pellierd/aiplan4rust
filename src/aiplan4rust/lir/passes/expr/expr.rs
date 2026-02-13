use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::logic::LogicError;
use crate::aiplan4rust::lir::logic::rewrite::eliminate_imply;
use crate::aiplan4rust::lir::logic::rewrite::factorize_time_specifier;
use crate::aiplan4rust::lir::logic::rewrite::push_negation;
use crate::aiplan4rust::lir::logic::rewrite::push_time_specifier;
use crate::aiplan4rust::lir::logic::simplify::simplify;

/// Simplifies a PDDL-like expression tree in a post-order traversal.
///
/// This function performs a **full simplification pass** over the given expression tree.
/// It traverses the tree in **post-order** (children before parent) and applies
/// node-specific simplification functions (`simplify_node`) to each node.
///
/// # Parameters
/// - `expr`: a mutable reference to the expression tree (`Expr`) to be simplified.
///
/// # Behavior
/// 1. Retrieves the root of the expression tree. If the tree is empty (`root_id` is `None`), the function returns immediately.
/// 2. Performs a **depth-first search (DFS)** in post-order using an explicit stack to avoid recursion:
///     - Each stack entry is `(node_id, visited)` where `visited` indicates if children have already been processed.
///     - Children are pushed first, then the parent is revisited to ensure post-order processing.
/// 3. After constructing the post-order list of node IDs, each node is simplified by calling `simplify_node(node_id, expr)`.
///
/// # Returns
/// - `Ok(())` if the simplification completes successfully.
/// - `Err(ExprError)` if any node access or mutation fails during traversal or simplification.
///
/// # Notes
/// - Post-order traversal ensures that child nodes are simplified before their parents, which
///   is critical for transformations like flattening, deduplication, and reducing single-child AND/OR nodes.
/// - This function does not modify the tree if it is empty.
/// - Simplification logic for each node type is delegated to `simplify_node`.
///
/// # Example
/// ```ignore
/// let mut expr = build_expr_tree(); // some Expr tree
pub fn normalize(expr: &mut Expr) -> Result<(), LogicError> {
    let Some(root_id) = expr.root_id() else { return Ok(()); };

    eliminate_imply(root_id, expr)?;
    push_negation(root_id, expr)?;

    if push_time_specifier(root_id, expr)? {
        factorize_time_specifier(root_id, expr)?;
    }

    simplify(root_id, expr, None)?;
    // TO ADD as post-processing afet simplify
    // Factorization
    // Example: `(A ∧ B) ∨ (A ∧ C) -> A ∧ (B ∨ C)`.
    Ok(())
}
