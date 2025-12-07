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
/// # Preconditions
/// For the simplification to work correctly, the expression tree must satisfy the following:
/// 1. **All `Imply` nodes have been removed** or transformed (e.g., `Imply(A, B) -> Or(Not(A), B)`).
///    `Imply` nodes are not handled in `simplify_node`.
/// 2. **Negations have been pushed down** to atomic formulas using De Morgan's laws and quantifier rules:
///    - `Not(And(A, B)) -> Or(Not(A), Not(B))`
///    - `Not(Or(A, B)) -> And(Not(A), Not(B))`
///    - `Not(Forall(vars, body)) -> Exists(vars, Not(body))`
///    - `Not(Exists(vars, body)) -> Forall(vars, Not(body))`
/// 3. **Temporal specifiers (`AtStart`, `AtEnd`, `Overall`) have been factorized** so that each modifier
///    appears only once at the top of conjunctions. For example:
///        ```lisp
///        (and (at-start (A)) (at-start (B)) (at-end (C)))
///        => (and (at-start (A) (B)) (at-end (C)))
///        ```
///
/// # Parameters
/// - `root_id`: The ID of the root node of the expression tree.
/// - `expr`: A mutable reference to the expression tree (`Expr`) to be simplified.
///
/// # Behavior
/// 1. Performs a **depth-first search (DFS)** in post-order using an explicit stack to avoid recursion:
///     - Each stack entry is `(node_id, visited)` where `visited` indicates if children have already been processed.
///     - Children are pushed first, then the parent is revisited to ensure post-order processing.
/// 2. After constructing the post-order list of node IDs, each node is simplified by calling `simplify_node(node_id, expr)`.
/// 3. Node-specific simplifications include:
///     - `And` / `Or`: flattening, deduplication, reducing single-child nodes.
///     - `Not`: already pushed down; can be further simplified if nested.
///     - `Forall` / `Exists`: body simplification and variable factoring.
///     - `When`: simplify conditional effects.
///     - Temporal nodes (`AtStart`, `AtEnd`, `Overall`): already factorized, nothing more to do.
///
/// # Returns
/// - `Ok(())` if the simplification completes successfully.
/// - `Err(ExprError)` if any node access or mutation fails during traversal or simplification.
///
/// # Notes
/// - Post-order traversal ensures that children are simplified before their parents,
///   which is essential for flattening, deduplication, and factorization.
/// - Nodes that do not require simplification (atomic formulas, types, constants, variables, etc.) are skipped.
///
/// # Example
/// ```ignore
/// let mut expr = build_expr_tree();
/// let root_id = expr.root_id().unwrap();
///
/// // Preconditions must be satisfied before calling simplify:
/// // - All Implies removed
/// // - Negations pushed down
/// // - Temporal specifiers factorized
///
/// simplify(root_id, &mut expr)?;
/// ```
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
        // Logical operators
        ExprKind::And | ExprKind::Or => and_or::simplify(node_id, expr)?,
        ExprKind::Not => not::simplify(node_id, expr)?,
        ExprKind::Forall | ExprKind::Exists => quantifier::simplify(node_id, expr)?,
        ExprKind::Assign => assign::simplify(node_id, expr)?,
        ExprKind::FComp => comparison::simplify(node_id, expr)?,
        ExprKind::Operation => arithmetic::simplify(node_id, expr)?,
        ExprKind::When => when::simplify(node_id, expr)?,

        // Nodes that should not appear here
        ExprKind::Imply => return Err(ExprError::invalid_expr_node(node_id, ExprKind::Imply)),

        // No simplification needed, post-order ensures children are already simplified
        Kind::AtStart | Kind::AtEnd | Kind::Overall
        | Kind::Always | Kind::Sometime | Kind::Within
        | Kind::AtMostOnce | Kind::SometimeAfter
        | Kind::SometimeBefore | Kind::AlwaysWithin
        | Kind::HoldDuring | Kind::HoldAfter => {}

        // Leaf nodes or nodes that don’t require simplification
        Kind::Type | Kind::TypedList | Kind::TypedSymbol | Kind::FunctionTerm | Kind::AtomicFormula
        | Kind::Number | Kind::Preference | Kind::Constant | Kind::Variable | Kind::FunctionSymbol
        | Kind::PrimitiveType | Kind::Predicate | Kind::TaskSymbol | Kind::PrefName
        | Kind::TimedInitialLiteral | Kind::Metric | Kind::TotalTime | Kind::IsViolated
        | Kind::Length | Kind::Serial | Kind::Parallel | Kind::Task | Kind::TaskID
        | Kind::TaggedTask | Kind::TaskOrderingConstraint => { },
    }

    Ok(())
}
