
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::lir::expr::{Expr, ExprNode, ExprContent, ExprKind, ExprId};

/// Applies negation pushing transformation starting from the root syntax.
/// Returns the ExprId of the transformed syntax.
pub fn push_negations(expr: &mut Expr) -> Result<ExprId, AiplanError> {
    transform_node(expr.try_root_id()?, expr, false)
}

/// Recursively transforms a syntax according to the accumulated negation state.
///
/// If `negated` is true, negation is pushed down on this syntax.
fn transform_node(
    id: ExprId,
    expr: &mut Expr,
    negated: bool,
) -> Result<ExprId, AiplanError> {
    // First borrow immutably to get needed data, then drop borrow before mutable borrow
    let (kind, children, parent) = {
        let node = expr.try_node(id)?;
        (node.kind(), node.children().to_vec(), node.parent())
    };

    match kind {
        ExprKind::Not => {
            // Flip negation and transform child
            let child_id = children[0];
            transform_node(child_id, expr, !negated)
        }
        ExprKind::And | ExprKind::Or => {
            // Apply De Morgan’s law if negated
            let new_kind = apply_demorgan(kind, negated);

            // Create new syntax with the new kind
            let new_node = ExprNode::new(new_kind, ExprContent::None, parent);
            let new_id = expr.alloc(new_node);

            // Recursively transform each child with the current negation state
            for &child_id in &children {
                let transformed_child_id = transform_node(child_id, expr, negated)?;
                let new_node_mut = expr.try_node_mut(new_id)?;
                new_node_mut.add_child(transformed_child_id);
            }

            Ok(new_id)
        }
        ExprKind::Predicate
        | ExprKind::Constant
        | ExprKind::Variable
        | ExprKind::FunctionSymbol => {
            // If negated, wrap syntax in a Not syntax
            if negated {
                wrap_not(id, expr)
            } else {
                Ok(id)
            }
        }
        _ => {
            // Return syntax as is for other kinds (can be extended)
            Ok(id)
        }
    }
}

/// Applies De Morgan’s law to AND/OR nodes under negation.
///
/// If `negated` is true, swaps AND and OR; otherwise returns `kind` unchanged.
fn apply_demorgan(kind: ExprKind, negated: bool) -> ExprKind {
    if negated {
        match kind {
            ExprKind::And => ExprKind::Or,
            ExprKind::Or => ExprKind::And,
            _ => kind,
        }
    } else {
        kind
    }
}

/// Wraps the given syntax in a Not syntax.
fn wrap_not(id: ExprId, expr: &mut Expr) -> Result<ExprId, AiplanError> {
    let not_node = ExprNode::new(ExprKind::Not, ExprContent::None, None);
    let not_id = expr.alloc(not_node);
    let not_node_mut = expr.try_node_mut(not_id)?;
    not_node_mut.add_child(id);
    Ok(not_id)
}
