use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lang::BinaryComp;
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxContent};
use ordered_float::OrderedFloat;

/// Simplifies an FComp node by applying all simplification strategies in sequence:
/// 1. Constant evaluation
/// 2. Identity simplification (x = x, f(x) = f(x), x >= x, etc.)
/// 3. Trivial/redundant comparisons
///
/// Returns `true` if the node was simplified, `false` otherwise.
pub fn simplify_fcomp(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    let node = expr.try_node(node_id)?;

    if node.kind() != ExprKind::FComp {
        return Ok(false);
    }

    // Try constant evaluation first
    if simplify_comparison_constants(node_id, expr)? {
        return Ok(true);
    }

    // Then try identity simplification (variable = variable)
    if simplify_comparison_trivial_identity(node_id, expr)? {
        return Ok(true);
    }

    Ok(false)
}

/// Simplifies a `Comparison` node if both operands are constant numeric values.
///
/// This function evaluates a binary comparison (`BinaryComp`) between two constant numbers
/// (`Float` nodes). If both children of the node are constants, it replaces the `Comparison`
/// node with:
/// - `ExprKind::And` if the comparison evaluates to `true` (always satisfied),
/// - `ExprKind::Or` if the comparison evaluates to `false` (never satisfied).
///
/// # Parameters
///
/// * `node_id` - The ID of the comparison node to simplify.
/// * `expr` - A mutable reference to the expression tree (`Expr`) containing the node.
///
/// # Returns
///
/// * `Ok(true)` if the node was successfully simplified.
/// * `Ok(false)` if the node could not be simplified (wrong kind, missing `BinaryComp` content,
///    wrong number of children, or non-constant children).
/// * `Err(ExprError)` if node access or mutation fails.
///
/// # Notes
///
/// - This function only operates on nodes of kind `ExprKind::FComp`.
/// - The simplification is safe and deterministic because it only evaluates constant values.
/// - After simplification, the node has no children and its content is set to `Content::None`.
fn simplify_comparison_constants(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    let node = expr.try_node(node_id)?;

    // Ensure the node is of type FComp
    if node.kind() != ExprKind::FComp {
        return Ok(false);
    }

    let op = match node.content().as_binary_comp() {
        Some(op) => op,
        None => {
            debug_assert!(false, "FComp node without BinaryComp content");
            return Ok(false); // should not happen in normal usage
        }
    };

    let children = node.children();
    if children.len() != 2 {
        debug_assert!(children.len() == 2, "FComp node does not have exactly 2 children");
        return Ok(false);
    }

    // Try to get constant values from both children
    let left_val = match expr.try_node(children[0])?.content().as_float() {
        Some(v) => v,
        None => return Ok(false),
    };
    let right_val = match expr.try_node(children[1])?.content().as_float() {
        Some(v) => v,
        None => return Ok(false),
    };

    // Evaluate the comparison directly on OrderedFloat
    let result = match op {
        BinaryComp::Equal => left_val == right_val,
        BinaryComp::Greater => left_val > right_val,
        BinaryComp::Less => left_val < right_val,
        BinaryComp::GreaterEq => left_val >= right_val,
        BinaryComp::LessEq => left_val <= right_val,
    };

    // Replace the node with an always true (and) or always false (or) node
    let mut node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_kind(if result { ExprKind::And } else { ExprKind::Or });
    node_mut.set_content(Content::None);
    node_mut.set_children(vec![]);

    Ok(true)
}


/// Simplifies an FComp node when both children are trivially identical.
///
/// This function handles comparisons where the left and right children are exactly the same:
/// - `= x x` or `= f(x) f(x)` → always true → replaced with `and`
/// - `>= x x` or `<= x x` → always true → replaced with `and`
/// - `< x x` or `> x x` → always false → replaced with `or`
///
/// # Arguments
///
/// * `node_id` - The ID of the FComp node to simplify.
/// * `expr` - Mutable reference to the expression tree containing the node.
///
/// # Returns
///
/// * `Ok(true)` if the node was simplified.
/// * `Ok(false)` if no simplification was possible (different terms, not FComp, etc.).
/// * `Err(ExprError)` if accessing or mutating the node fails.
pub fn simplify_comparison_trivial_identity(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprError> {
    let node = expr.try_node(node_id)?;

    // Only operate on FComp nodes
    if node.kind() != ExprKind::FComp {
        return Ok(false);
    }

    let children = node.children();
    if children.len() != 2 {
        debug_assert!(children.len() == 2, "FComp node does not have exactly 2 children");
        return Ok(false);
    }

    let left = expr.try_node(children[0])?;
    let right = expr.try_node(children[1])?;

    // Check if left and right are exactly the same node (variable or function)
    if left == right {
        let op = node.content().as_binary_comp();
        let is_true = matches!(
            op,
            Some(BinaryComp::Equal)
                | Some(BinaryComp::GreaterEq)
                | Some(BinaryComp::LessEq)
        );

        let mut node_mut = expr.try_node_mut(node_id)?;
        node_mut.set_kind(if is_true { ExprKind::And } else { ExprKind::Or });
        node_mut.set_content(Content::None);
        node_mut.set_children(vec![]);
        return Ok(true);
    }

    Ok(false)
}
