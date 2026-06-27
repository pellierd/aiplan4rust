use crate::aiplan4rust::compiler::lir::expr::iter::PreorderIter;
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprStore};

/// Vérifie si une expression respecte strictement la Negation Normal Form (NNF).
///
/// En NNF :
/// - L'opérateur `Imply` doit avoir été éliminé.
/// - L'opérateur `Not` ne peut se trouver **que** directement au-dessus d'une
///   `AtomicFormula` ou d'une `Comparison`.
pub fn is_nnf(store: &ExprStore, root: ExprId) -> bool {
    if root.is_none() {
        return true;
    }

    // 🏎️ On transforme l'itérateur pour récupérer des ExprNode propres
    let mut tree_iter = PreorderIter::new(store, root).references();

    while let Some(node) = tree_iter.next() {
        match node.kind() {
            ExprKind::Imply => return false,
            ExprKind::Not => {
                if let Some(&child_id) = node.children().first() {
                    if child_id.is_valid() {
                        let child_kind = store[child_id].kind();
                        if !matches!(
                            child_kind,
                            ExprKind::AtomicFormula(_) | ExprKind::Comparison(_)
                        ) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

/// Vérifie si une expression est en Quantifier Normal Form (QNF) / Grounded.
///
/// En QNF, l'expression est totalement instanciée (grounded), ce qui signifie
/// qu'aucun quantificateur (`forall`, `exists`) ne doit subsister dans l'arbre.
pub fn is_qnf(store: &ExprStore, root: ExprId) -> bool {
    if root.is_none() {
        return true;
    }

    for (_, _, entry) in store.preorder(root) {
        match entry.kind() {
            ExprKind::ForallNew(_) | ExprKind::ExistsNew(_) => return false,
            _ => {}
        }
    }
    true
}
