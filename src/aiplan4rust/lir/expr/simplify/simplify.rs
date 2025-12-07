use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};
use crate::aiplan4rust::lir::expr::kind::Kind;
use crate::aiplan4rust::lir::expr::simplify::{and_or, arithmetic, assign, comparison, not, quantifier, when};
use crate::aiplan4rust::syntax::tree::NodeId;

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
pub fn simplify(root_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {

    // Stack pour DFS post-order: (node_id, visited)
    let mut stack = vec![(root_id, false)];
    let mut postorder = Vec::new();

    while let Some((node_id, visited)) = stack.pop() {
        if visited {
            postorder.push(node_id);
        } else {
            stack.push((node_id, true));
            for &child_id in expr.try_node(node_id)?.children() {
                stack.push((child_id, false));
            }
        }
    }

    for node_id in postorder {
        simplify_node(node_id, expr)?;
    }

    /// TO ADD
    /// Factorise les parties commpostunes des expressions.
    /// Exemple: `(A ∧ B) ∨ (A ∧ C) -> A ∧ (B ∨ C)`.
    Ok(())
}

/// Simplifies a node in a PDDL expression tree based on its kind.
///
/// This function inspects the type of the node identified by `node_id` and applies
/// the appropriate simplification routine for that type. Currently, it only handles
/// `AND` and `OR` nodes by delegating to `simplify_and_or_node`.
/// Nodes of other kinds are left unchanged.
///
/// # Parameters
/// - `node_id`: The ID of the node to simplify.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if the simplification succeeds or the node type is not handled.
/// - `Err(ExprError)` if accessing the node fails.
///
/// # Notes
/// - This function is intended to be called from a post-order traversal of the
///   expression tree, so that children are simplified before their parent.
/// - Extending this function to support additional node kinds (e.g., `NOT`,
///   arithmetic expressions) is straightforward: simply add a match arm
///   for the new kind.
///
/// # Example
/// ```ignore
/// let node_id = expr.root_id().unwrap();
/// simplify_node(node_id, &mut expr)?;
/// ```
#[allow(dead_code)]
fn simplify_node(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {

    let kind = expr.try_node(node_id)?.kind();

    match kind {
        ExprKind::And | ExprKind::Or => {
            and_or::simplify(node_id, expr)?;
        }
        ExprKind::Not => {
            not::simplify(node_id, expr)?;
        }
        ExprKind::Forall | ExprKind::Exists => {
            quantifier::simplify(node_id, expr)?;
        }
        ExprKind::Imply => {
            return Err(ExprError::InvalidExprNode {
                node_id,
                kind: ExprKind::Imply,
            });
        }
        ExprKind::Assign => {
            assign::simplify(node_id, expr)?;
        }
        ExprKind::FComp => {
            comparison::simplify(node_id, expr)?;
        }
        ExprKind::Operation => {
            arithmetic::simplify(node_id, expr)?;
        }
        ExprKind::When => {
            when::simplify(node_id, expr)?;
        }


        Kind::Type => {}
        Kind::TypedList => {}
        Kind::AtStart => {}
        Kind::AtEnd => {}
        Kind::Overall => {}
        Kind::Always => {}
        Kind::Sometime => {}
        Kind::Within => {}
        Kind::AtMostOnce => {}
        Kind::SometimeAfter => {}
        Kind::SometimeBefore => {}
        Kind::AlwaysWithin => {}
        Kind::HoldDuring => {}
        Kind::HoldAfter => {}

        // RIEN A FAIRE
        Kind::TypedSymbol => {}
        Kind::FunctionTerm => {}
        Kind::AtomicFormula => {}
        Kind::Number => {}
        Kind::Preference => {}
        Kind::Constant => {}
        Kind::Variable => {}
        Kind::FunctionSymbol => {}
        Kind::PrimitiveType => {}
        Kind::Predicate => {}
        Kind::TaskSymbol => {}
        Kind::PrefName => {}
        Kind::TimedInitialLiteral => {}
        Kind::Metric => {}
        Kind::TotalTime => {}
        Kind::IsViolated => {}
        Kind::Length => {}
        Kind::Serial => {}
        Kind::Parallel => {}
        Kind::Task => {}
        Kind::TaskID => {}
        Kind::TaggedTask => {}
        Kind::TaskOrderingConstraint => {}
    }

    Ok(())
}
