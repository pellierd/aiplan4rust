use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};
use crate::aiplan4rust::syntax::tree::NodeId;

///
/// Simplification rules:
/// 1. `(when (and) E) -> E`
/// 2. `(when (or) E) -> (and)`
/// 3. `(when E E) -> (and)`
/// 4. `(when C (and)) -> (and)`
///
/// # Parameters
/// - `node_id`: The ID of the `When` node to normalize.
/// - `expr`: Mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(true)` if the node was simplified.
/// - `Ok(false)` if no simplification was done.
/// - `Err(ExprError)` if node access fails.
/// Simplifies a `When` node in the expression tree according to PDDL rules.
pub fn simplify_when_node(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    let node = expr.try_node(node_id)?;
    debug_assert!(node.kind() == ExprKind::When, "Node must be a When");

    let children = node.children();
    if children.len() != 2 {
        return Ok(false);
    }

    let cond_id = children[0];
    let eff_id = children[1];

    let cond = expr.try_node(cond_id)?;
    let eff = expr.try_node(eff_id)?;

    // Case 1: (when (and) E) -> E
    if cond.kind() == ExprKind::And {
        expr.move_to(node_id, eff_id)?;
        return Ok(true);
    }

    // Case 2: (when (or) E) -> (and)
    if cond.kind() == ExprKind::Or {
        expr.set_empty_and(node_id)?;
        return Ok(true);
    }

    // Case 3: (when E E) -> (and)
    if cond_id == eff_id {
        expr.set_empty_and(node_id)?;
        return Ok(true);
    }

    // Case 4: (when C (and)) -> (and)
    if eff.kind() == ExprKind::And {
        expr.set_empty_and(node_id)?;
        return Ok(true);
    }

    Ok(false)
}
