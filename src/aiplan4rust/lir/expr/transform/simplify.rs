use std::collections::HashSet;
use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};

/// Simplifies an expression by flattening nested AND/OR nodes and
/// reducing single-child logical nodes.
///
/// # Arguments
/// * `expr` - The expression to simplify.

pub fn simplify(expr: &mut Expr) -> Result<(), ExprError> {
    let root_id = match expr.root_id() {
        Some(id) => id,
        None => return Ok(()), // tree empty, nothing to do
    };

    let mut stack = vec![root_id];

    while let Some(node_id) = stack.pop() {
        let node = expr.try_node(node_id)?;
        let node_kind = node.kind();

        if node_kind == ExprKind::And || node_kind == ExprKind::Or {
            let mut tmp_children = Vec::new();
            let mut seen_ids = HashSet::new();

            // Instead of cloning, we take ownership of the children
            let children_ids = std::mem::take(expr.try_node_mut(node_id)?.children_mut());

            for child_id in children_ids {
                let child_node = expr.try_node(child_id)?;
                if child_node.kind() == node_kind {
                    // Flatten grandchildren directly
                    let grand_children = std::mem::take(expr.try_node_mut(child_id)?.children_mut());
                    for grandchild_id in grand_children {
                        if seen_ids.insert(grandchild_id) {
                            tmp_children.push(grandchild_id);
                        }
                    }
                } else if seen_ids.insert(child_id) {
                    tmp_children.push(child_id);
                }
            }

            // Replace children with flattened result
            expr.try_node_mut(node_id)?.set_children(tmp_children);
        }

        // Push children for postorder processing
        stack.extend(expr.try_node(node_id)?.children());
    }

    // Adjust root if it has a single child after simplification
    if let Some(root_id) = expr.root_id() {
        let root_node = expr.try_node(root_id)?;
        if matches!(root_node.kind(), ExprKind::And | ExprKind::Or)
            && root_node.children().len() == 1
        {
            expr.set_root_id(root_node.children()[0])?;
        }
    }

    Ok(())
}





/// Élimine les constantes `true` / `false` dans les AND/OR.
/// Exemple: `(and A true) -> A`, `(or A false) -> A`.
pub fn eliminate_constants(expr: &mut Expr) {
    // TODO: implémenter la simplification des constantes
}

/// Nettoyage final après toutes les transformations
/// Supprime les nœuds vides et triviales restantes.
pub fn simplify_final(expr: &mut Expr) {
    // TODO: implémenter nettoyage final
}


#[cfg(test)]
mod tests {
    use std::fmt::Display;
    use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
    use super::*;
    use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprNode, NodeId};
    use crate::aiplan4rust::lang::Ident;
    use crate::aiplan4rust::syntax::SyntaxDisplay;

    /// Alloue un nœud prédicat
    fn alloc_predicate(
        expr: &mut Expr,
        interner: &mut StringInterner,
        name: &str
    ) -> NodeId {
        let ident = interner.intern_ident(name.to_string());
        let node = ExprNode::new(
            ExprKind::Predicate,
            ExprContent::Ident(ident),
            None,
        );
        expr.alloc(node)
    }

    /// Alloue un nœud AND avec des enfants donnés
    fn alloc_and(expr: &mut Expr, children: Vec<NodeId>) -> NodeId {
        let node = ExprNode::new(
            ExprKind::And,
            ExprContent::None,
            None,
        );
        expr.alloc_with_children(node, children)
    }

    /// Alloue un nœud OR avec des enfants donnés
    fn alloc_or(expr: &mut Expr, children: Vec<NodeId>) -> NodeId {
        let node = ExprNode::new(
            ExprKind::Or,
            ExprContent::None,
            None,
        );
        expr.alloc_with_children(node, children)
    }

    #[test]
    fn test_simplify_nested_and() {
        println!("\n=== Test: simplify_nested_and ===");

        // Interner local au test
        let mut interner = StringInterner::new();

        let mut expr = Expr::new();

        // Créer des variables A, B, C
        let a = alloc_predicate(&mut expr, &mut interner, "A");
        let b = alloc_predicate(&mut expr, &mut interner, "B");
        let c = alloc_predicate(&mut expr, &mut interner, "C");

        // Créer (and A B)
        let inner_and = alloc_and(&mut expr, vec![a, b]);

        // Créer (and (and A B) C)
        let root = alloc_and(&mut expr, vec![inner_and, c]);
        expr.set_root_id(root).unwrap();

        // --- Affichage avant simplification ---
        print!("{}", expr.to_syntax_string(&interner));

        // Appliquer la simplification
        print!(" → ");
        simplify(&mut expr).unwrap();

        // --- Affichage après simplification ---
        let new_root = expr.root_id().unwrap();
        println!("{}", expr.to_syntax_string(&interner));

        // Vérifier que l'arbre a été aplati : (and A B C)
        let root_node = expr.try_node(new_root).unwrap();
        assert_eq!(root_node.children().len(), 3);

        let child_kinds: Vec<_> = root_node.children()
            .iter()
            .map(|&id| expr.try_node(id).unwrap().kind())
            .collect();

        assert_eq!(
            child_kinds,
            vec![ExprKind::Predicate, ExprKind::Predicate, ExprKind::Predicate]
        );
    }
}
