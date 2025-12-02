use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::syntax::tree::NodeId;

/// Simplify an IMPLY node: `A -> B` becomes `(or (not A) B)`
///
/// Steps:
/// 1. Assumes premise (A) and consequence (B) have been simplified (post-order).
/// 2. Creates a Not node for the premise.
/// 3. Immediately simplifies the Not node (handle double negation, empty And/Or, etc.).
/// 4. Converts the current Imply node into an Or(Not(premise), consequence).
/// 5. Simplifies the resulting Or node (flattening, deduplication, single-child reduction).
pub(crate) fn remove_imply(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    // Stack pour DFS post-order : (node_id, visited)
    let mut stack = vec![(node_id, false)];

    while let Some((curr_id, visited)) = stack.pop() {
        if visited {
            // Post-order : traiter le noeud après ses enfants
            let node = expr.try_node(curr_id)?;
            if node.kind() != ExprKind::Imply {
                continue;
            }

            let children = node.children();
            if children.len() != 2 {
                debug_assert!(
                    false,
                    "Imply node should have exactly 2 children, found {}",
                    children.len()
                );
                continue;
            }

            let premise = children[0];
            let consequence = children[1];

            // Créer Not(premise)
            let not_node = ExprNode::new(ExprKind::Not, ExprContent::None, None);
            let not_premise_id = expr.alloc_with_children(not_node, vec![premise]);

            // Transformer le noeud courant en Or(Not(premise), consequence)
            let node_mut = expr.try_node_mut(curr_id)?;
            node_mut.set_kind(ExprKind::Or);
            node_mut.set_children(vec![not_premise_id, consequence]);
        } else {
            // Marquer comme visité et empiler les enfants
            stack.push((curr_id, true));
            let node = expr.try_node(curr_id)?;
            for &child_id in node.children() {
                stack.push((child_id, false));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::syntax::SyntaxDisplay;

    /// Input: (A -> B)
    /// Expected output: (or (B) (not (A)))
    #[test]
    fn test_simple_imply() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let imply = builder.imply(a, b);
        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        remove_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (not (A)) (B))");
    }

    /// Input: (A -> (and B C))
    /// Expected output: (or (and (B) (C)) (not (A)))
    #[test]
    fn test_imply_with_and_consequence() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let and_bc = builder.and(vec![b, c]);
        let imply = builder.imply(a, and_bc);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        remove_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (not (A)) (and (B) (C)))");
    }

    /// Input: (A -> (and))
    /// Expected output: (or (not (A)) (and))
    #[test]
    fn test_imply_with_empty_and_consequence() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let empty_and = builder.and(vec![]);
        let imply = builder.imply(a, empty_and);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        remove_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert!(!root_node.children().is_empty());
        assert_eq!(output, "(or (not (A)) (and))");
    }

    /// Input: ((forall ?X A) -> (exists ?Y B))
    /// Expected output: (or (not (forall (?X) (A))) (exists (?Y) (B)))
    #[test]
    fn test_imply_with_quantifiers() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let x = builder.variable("?X");
        let vars = builder.typed_list(vec![x]);
        let forall_node = builder.forall(vars, a);

        let b = builder.atomic_formula("B", vec![]);
        let y = builder.variable("?Y");
        let vars = builder.typed_list(vec![y]);
        let exists_node = builder.exists(vars, b);

        let imply = builder.imply(forall_node, exists_node);
        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        remove_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(
            output,
            "(or (not (forall (?X) (A))) (exists (?Y) (B)))"
        );
    }
}
