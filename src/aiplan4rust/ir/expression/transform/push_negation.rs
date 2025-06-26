use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::ir::expression::{Expr, ExprContent, ExprKind};
use crate::aiplan4rust::tree::{TreeArena, NodeId};

/// Applique la transformation de poussée des négations à partir d'un noeud racine.
/// Retourne le NodeId du noeud transformé.
pub fn push_negations(root_id: NodeId, arena: &mut TreeArena<Expr>) ->  Result<NodeId, ParserInternalError> {
    transform_node(root_id, arena, false)
}

/// Transforme un noeud donné en fonction de la négation accumulée.
///
/// `negated == true` signifie que la négation doit être poussée sur ce noeud.
fn transform_node(
    node_id: NodeId,
    arena: &mut TreeArena<Expr>,
    negated: bool,
) -> Result<NodeId, ParserInternalError> {
    // On récupère d’abord les infos nécessaires puis on libère l'emprunt immuable
    let (kind, children, parent) = {
        let node = arena.try_node(node_id)?;
        (node.kind(), node.children().to_vec(), node.parent())
    };

    match kind {
        ExprKind::Not => {
            // Inverse l'état de négation et transforme l'enfant
            let child_id = children[0];
            transform_node(child_id, arena, !negated)
        }
        ExprKind::And | ExprKind::Or => {
            // Applique la loi de De Morgan si négation
            let new_kind = apply_demorgan(kind, negated);

            // Crée un nouveau noeud avec le nouveau kind
            let mut new_expr = Expr::new(new_kind, ExprContent::None, parent);
            let new_node_id = arena.add(new_expr);

            // Transforme chaque enfant avec la même négation (ou non)
            for &child_id in &children {
                let transformed_child_id = transform_node(child_id, arena, negated)?;
                let new_expr = arena.try_node_mut(new_node_id)?;
                new_expr.add_child(transformed_child_id);
            }

            Ok(new_node_id)
        }
        ExprKind::Predicate
        | ExprKind::Constant
        | ExprKind::Variable
        | ExprKind::FunctionSymbol => {
            // Si on est sous une négation, crée un noeud Not au-dessus
            if negated {
                wrap_not(node_id, arena)
            } else {
                Ok(node_id)
            }
        }
        _ => {
            // Pour les autres cas, retourne le noeud tel quel (optionnel à étendre)
            Ok(node_id)
        }
    }
}

/// Applique la loi de De Morgan pour AND/OR sous négation.
/// Si negated est vrai, AND <-> OR, sinon retourne kind inchangé.
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

/// Crée un noeud Not qui enveloppe le noeud donné.
fn wrap_not(node_id: NodeId, arena: &mut TreeArena<Expr>) -> Result<NodeId, ParserInternalError> {
    let mut not_expr = Expr::new(ExprKind::Not, ExprContent::None, None);
    let not_node_id = arena.add(not_expr);
    let not_node = arena.try_node_mut(not_node_id)?;
    not_node.add_child(node_id);
    Ok(not_node_id)
}
