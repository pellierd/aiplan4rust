use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::transform::simplify::and_or::simplify_and_or;
use crate::aiplan4rust::lir::expr::transform::simplify::not::simplify_not;
use crate::aiplan4rust::syntax::tree::NodeId;

/// Simplify an IMPLY node: `A -> B` becomes `(or (not A) B)`
///
/// Steps:
/// 1. Assumes premise (A) and consequence (B) have been simplified (post-order).
/// 2. Creates a Not node for the premise.
/// 3. Immediately simplifies the Not node (handle double negation, empty And/Or, etc.).
/// 4. Converts the current Imply node into an Or(Not(premise), consequence).
/// 5. Simplifies the resulting Or node (flattening, deduplication, single-child reduction).
pub(in crate::aiplan4rust::lir::expr::transform::simplify) fn simplify_imply(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    let node = expr.try_node(node_id)?;
    if node.kind() != ExprKind::Imply {
        return Ok(()); // skip non-imply nodes
    }

    // Ensure Imply node has exactly two children
    let children = node.children();
    if children.len() != 2 {
        // Debug info for invalid Imply node
        debug_assert!(
            false,
            "Imply node should have exactly 2 children, found {}",
            children.len()
        );
        return Ok(()); // Do nothing if the node is malformed
    }

    let premise = children[0];
    let consequence = children[1];

    // Step 1: Create Not(premise)
    let not_node = ExprNode::new(ExprKind::Not, ExprContent::None, None);
    let not_premise = expr.alloc_with_children(not_node, vec![premise]);

    // Step 2: Simplify the Not node immediately
    simplify_not(not_premise, expr)?;

    // Step 3: Convert the current node into Or(Not(premise), consequence)
    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_kind(ExprKind::Or);
    node_mut.set_children(vec![not_premise, consequence]);

    // Step 4: Simplify the new Or node
    simplify_and_or(node_id, expr)?;

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
    /// Expected output: (or (not (A)) (B))
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
        simplify_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (B) (not (A)))");
    }

    /// Input: ((not (not A)) -> B)
    /// Expected output: (or (not (A)) (B))
    #[test]
    fn test_imply_with_double_negation_premise() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let not_a = builder.not(a);
        let double_not_a = builder.not(not_a);
        let b = builder.atomic_formula("B", vec![]);
        let imply = builder.imply(double_not_a, b);
        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (B) (not (A)))"); // after simplification, double negation removed
    }

    /// Input: (A -> (and B C))
    /// Expected output: (or (not (A)) (and (B) (C)))
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
        simplify_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (and (B) (C)) (not (A)))");
    }

    /// Input: ((or A B) -> C)
    /// Expected output: (or (not (or (A) (B))) (C))
    #[test]
    fn test_imply_with_or_premise() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let or_ab = builder.or(vec![a, b]);
        let c = builder.atomic_formula("C", vec![]);
        let imply = builder.imply(or_ab, c);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (C) (not (or (A) (B))))");
    }

    /// Input: (A -> (and))
    /// Expected output: (and)
    #[test]
    fn test_imply_with_empty_and_consequence() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let empty_and = builder.and(vec![]); // consequence
        let imply = builder.imply(a, empty_and);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(output, "(and)");
    }

    /// Input: (A -> (or))
    /// Expected output: (not (A))
    #[test]
    fn test_imply_with_empty_or_consequence() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let empty_or = builder.or(vec![]); // consequence
        let imply = builder.imply(a, empty_or);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Not);
        assert_eq!(root_node.children().len(), 1);
        assert_eq!(output, "(not (A))");
    }

    /// Input: ((or) -> A)
    /// Expected output: (and)
    #[test]
    fn test_imply_with_empty_or_premise() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_or = builder.or(vec![]);
        let b = builder.atomic_formula("A", vec![]);
        let imply = builder.imply(empty_or, b);
        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        // Verify that the root node has been simplified to an empty AND
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(and)");
    }

    /// Input: ((and) -> A)
    /// Expected output: (A)
    #[test]
    fn test_imply_with_empty_and_premise() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_and = builder.and(vec![]);
        let b = builder.atomic_formula("A", vec![]);
        let imply = builder.imply(empty_and, b);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&mut interner);
        simplify_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&mut interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);
        assert_eq!(output, "(A)");
    }

    /// Input: ((forall ?X A) -> B)
    /// Expected output: (or (not (forall (?X) (A))) (B))
    #[test]
    fn test_imply_with_forall_premise() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let x = builder.variable("?X");
        let vars = builder.typed_list(vec![x]);
        let forall_node = builder.forall(vars, a);
        let b = builder.atomic_formula("B", vec![]);
        let imply = builder.imply(forall_node, b);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (B) (not (forall (?X) (A))))");
    }

    /// Input: (A -> (exists ?Y B))
    /// Expected output: (or (not (A)) (exists (?Y) (B)))
    #[test]
    fn test_imply_with_exists_consequence() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let y = builder.variable("?Y");
        let vars = builder.typed_list(vec![y]);
        let exists_node = builder.exists(vars, b);
        let imply = builder.imply(a, exists_node);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (exists (?Y) (B)) (not (A)))");
    }

    /// Input: ((forall ?X A) -> (exists ?Y B))
    /// Expected output: (or (not (forall (?X) (A))) (exists (?Y) (B)))
    #[test]
    fn test_imply_with_quantifiers_both_sides() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let x = builder.variable("?X");
        let vars_x = builder.typed_list(vec![x]);
        let forall_node = builder.forall(vars_x, a);

        let b = builder.atomic_formula("B", vec![]);
        let y = builder.variable("?Y");
        let vars_y = builder.typed_list(vec![y]);
        let exists_node = builder.exists(vars_y, b);

        let imply = builder.imply(forall_node, exists_node);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_imply(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(
            output,
            "(or (exists (?Y) (B)) (not (forall (?X) (A))))"
        );
    }
}
