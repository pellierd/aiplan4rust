use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::syntax::tree::NodeId;

/// Simplifies a `Not` node in a PDDL expression tree.
///
/// Currently, this only handles:
/// - Double negation: `(not (not X))` → `X`
/// - Trivial negation over empty AND/OR nodes: `(not (and))` → `(or)`, `(not (or))` → `(and)`
pub(in crate::aiplan4rust::lir::expr::transform::simplify) fn simplify_not(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), ExprError> {
    simplify_double_negation(node_id, expr)?;
    simplify_trivial_constant(node_id, expr)?;
    Ok(())
}


/// Simplifies a double negation in a PDDL expression tree.
///
/// This function transforms a `Not` node that has exactly one child,
/// where the child is also a `Not`, into the grandchild node, effectively
/// eliminating the double negation. The transformation is done in-place
/// without cloning the subtree, using `std::mem::take()` to move content and children.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the node to simplify. Must be a `Not` node.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Behavior
/// - If the node is not a `Not`, the function returns `Ok(())` and does nothing.
/// - If the node has zero or multiple children, it returns `Ok(())`.
/// - If the child is not a `Not` or has multiple children, it returns `Ok(())`.
/// - If the node has exactly one child, and that child is a `Not` with exactly
///   one child, the double negation is removed and the current node is replaced
///   by the grandchild node (kind, content, and children are moved).
///
/// # Returns
/// - `Ok(())` if the simplification completes successfully or if no simplification
///   is applicable.
/// - `Err(ExprError)` if accessing nodes or mutating the tree fails.
///
/// # Notes
/// - The function assumes the AST is generally well-formed (each `Not` should have
///   exactly one child). `debug_assert!` checks this invariant in debug builds.
/// - This is intended to be called as part of a post-order traversal of the expression tree.
///
/// # Example
/// ```text
/// Input:  (not (not X))
/// Output: X
/// ```
fn simplify_double_negation(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    let node = expr.try_node(node_id)?;
    if node.kind() != ExprKind::Not {
        return Ok(()); // Not a NOT node, ignore
    }

    debug_assert!(
        node.children().len() == 1,
        "Not node must have exactly one child"
    );
    let children = node.children();
    if children.len() != 1 {
        return Ok(()); // safety guard
    }

    let child_id = children[0];
    let child = expr.try_node(child_id)?;
    debug_assert!(
        child.children().len() == 1 || child.kind() != ExprKind::Not,
        "If child is a NOT, it must have exactly one child"
    );
    if child.kind() != ExprKind::Not || child.children().len() != 1 {
        return Ok(()); // only simplify double negation
    }

    let grandchild_id = child.children()[0];
    let grandchild_node = {
        let grandchild_mut = expr.try_node_mut(grandchild_id)?;
        let kind = grandchild_mut.kind();
        let content = std::mem::take(grandchild_mut.content_mut());
        let children = std::mem::take(grandchild_mut.children_mut());
        (kind, content, children)
    };

    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_kind(grandchild_node.0);
    node_mut.set_content(grandchild_node.1);
    node_mut.set_children(grandchild_node.2);

    Ok(())
}

/// Simplifies trivial constant expressions under a `Not` node.
///
/// Specifically handles empty `And` and `Or` nodes:
/// - `(not (and))` → `(or)`
/// - `(not (or))` → `(and)`
///
/// # Parameters
/// - `node_id`: The `NodeId` of the `Not` node to simplify.
/// - `expr`: A mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(())` if the simplification is performed successfully or no simplification applies.
/// - `Err(ExprError)` if accessing or mutating nodes fails.
///
/// # Behavior
/// - Checks if the node is a `Not`. If not, does nothing.
/// - Verifies that the `Not` node has exactly one child (debug assertion).
/// - If the child is an empty `And` or `Or`, flips it:
///   - `And` → `Or`
///   - `Or` → `And`
/// - Clears the children and sets content to `None` after simplification.
fn simplify_trivial_constant(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    // 1. Get the node
    let node = expr.try_node(node_id)?;
    if node.kind() != ExprKind::Not {
        return Ok(());
    }

    debug_assert!(node.children().len() == 1, "Not node must have exactly one child");
    if node.children().len() != 1 {
        return Ok(());
    }

    // 2. Get child id and kind before mutable borrow
    let child_id = node.children()[0];
    let child_kind;
    let child_empty;
    {
        let child = expr.try_node(child_id)?;
        child_kind = child.kind();
        child_empty = child.children().is_empty();
    }

    if !child_empty {
        return Ok(()); // only simplify empty And/Or
    }

    // 3. Mutable borrow pour modifier le Not
    let node_mut = expr.try_node_mut(node_id)?;
    match child_kind {
        ExprKind::And => node_mut.set_kind(ExprKind::Or),
        ExprKind::Or  => node_mut.set_kind(ExprKind::And),
        _ => return Ok(()),
    }
    node_mut.set_children(vec![]);
    node_mut.set_content(Content::None);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::syntax::SyntaxDisplay;

    /// Test simplification of a simple double negation: (not (not (A))) → (A)
    #[test]
    fn test_double_negation_simple_predicate() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let not1 = builder.not(atomic_a);
        let root = builder.not(not1);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_not(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);
        assert_eq!(output, "(A)");
    }

    /// Test simplification of a nested double negation containing a subtree.
    /// Input: (not (not (and (A) (B)))) -> (and (A) (B))
    #[test]
    fn test_double_negation_with_subtree() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let atomic_b = builder.atomic_formula("B", vec![]);
        let and = builder.and(vec![atomic_a, atomic_b]);
        let not1 = builder.not(and);
        let root = builder.not(not1);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_not(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);
        assert_eq!(output, "(and (A) (B))");
    }

    /// Test that no simplification is applied when negation is not doubled.
    /// Input: (not (and (A) (B))) -> unchanged
    #[test]
    fn test_not_node_no_simplification() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let atomic_b = builder.atomic_formula("B", vec![]);
        let and = builder.and(vec![atomic_a, atomic_b]);
        let root = builder.not(and);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_not(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Not);
        assert_eq!(output, "(not (and (A) (B)))");
    }

    /// Test simplification of `(not (and))` -> `(or)`
    #[test]
    fn test_not_empty_and_becomes_or() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_and = builder.and(vec![]);
        let root = builder.not(empty_and);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_not(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(or)");
    }

    /// Test simplification of `(not (or))` -> `(and)`
    #[test]
    fn test_not_empty_or_becomes_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_or = builder.or(vec![]);
        let root = builder.not(empty_or);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_not(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(and)");
    }

    /// Test that no simplification is applied on a NOT whose child is not a NOT or empty AND/OR.
    /// Input: (not (A)) -> unchanged
    #[test]
    fn test_not_other_operator_no_simplification() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let atomic_a = builder.atomic_formula("A", vec![]);
        let root = builder.not(atomic_a);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_not(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Not);
        assert_eq!(output, "(not (A))");
    }
}
