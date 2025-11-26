use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};
use crate::aiplan4rust::lir::expr::transform::simplify::and_or::simplify_and_or;
use crate::aiplan4rust::lir::expr::transform::simplify::not::simplify_not;
use crate::aiplan4rust::lir::expr::transform::simplify::quantifier::simplify_quantifier;
use crate::aiplan4rust::syntax::tree::NodeId;

/// Simplifies a PDDL-like expression tree in a post-order traversal.
///
/// This function performs a **full simplification pass** over the given expression tree.
/// It traverses the tree in **post-order** (children before parent) and applies
/// node-specific simplification functions (`simplify_node`) to each node.
///
/// # Parameters
/// - `expr`: a mutable reference to the expression tree (`Expr`) to be simplified.
///
/// # Behavior
/// 1. Retrieves the root of the expression tree. If the tree is empty (`root_id` is `None`), the function returns immediately.
/// 2. Performs a **depth-first search (DFS)** in post-order using an explicit stack to avoid recursion:
///     - Each stack entry is `(node_id, visited)` where `visited` indicates if children have already been processed.
///     - Children are pushed first, then the parent is revisited to ensure post-order processing.
/// 3. After constructing the post-order list of node IDs, each node is simplified by calling `simplify_node(node_id, expr)`.
///
/// # Returns
/// - `Ok(())` if the simplification completes successfully.
/// - `Err(ExprError)` if any node access or mutation fails during traversal or simplification.
///
/// # Notes
/// - Post-order traversal ensures that child nodes are simplified before their parents, which
///   is critical for transformations like flattening, deduplication, and reducing single-child AND/OR nodes.
/// - This function does not modify the tree if it is empty.
/// - Simplification logic for each node type is delegated to `simplify_node`.
///
/// # Example
/// ```ignore
/// let mut expr = build_expr_tree(); // some Expr tree
pub fn simplify(expr: &mut Expr) -> Result<(), ExprError> {
    let Some(root_id) = expr.root_id() else { return Ok(()); };

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
    /// Factorise les parties communes des expressions.
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
        ExprKind::And | ExprKind::Or => {
            simplify_and_or(node_id, expr)?;
        }
        ExprKind::Not => {
            simplify_not(node_id, expr)?;
        }
        ExprKind::Forall | ExprKind::Exists => {
            simplify_quantifier(node_id, expr)?;
        }
        _ => {} // Other node kinds are skipped
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

    /// Test flattening and structural deduplication on a complex nested AND node.
    ///
    /// Input: (and (and A B) (and B C) (and (and A B) D))
    /// Expected after simplify: (and A B C D)
    #[test]
    fn test_complex_nested_and_deduplication() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);
        let a = builder.predicate("A");
        let b = builder.predicate("B");
        let c = builder.predicate("C");
        let d = builder.predicate("D");
        let inner1 = builder.and(vec![a, b]);
        let inner2 = builder.and(vec![b, c]);
        let inner3 = builder.and(vec![inner1, d]);
        let root = builder.and(vec![inner1, inner2, inner3]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();
        let input = expr.to_syntax_string(&interner);
        simplify(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        let root_children: Vec<_> = root_node.children().iter().map(|&id| expr.try_node(id).unwrap().kind()).collect();
        assert!(root_children.iter().all(|&k| k == ExprKind::Predicate));
        assert_eq!(root_node.children().len(), 4);
    }
    /// Test simplification of a root AND with nested AND children and duplicates.
    ///
    /// Input: (and A (and B C) (and B C))
    /// Expected after simplify: (and A B C)
    #[test]
    fn test_root_and_structural_simplification() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.predicate("A");
        let b = builder.predicate("B");
        let c = builder.predicate("C");

        let inner1 = builder.and(vec![b, c]);
        let inner2 = builder.and(vec![b, c]);

        let root = builder.and(vec![a, inner1, inner2]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 3); // A, B, C
    }

    /// Test simplification of a root OR with nested OR children and duplicates.
    ///
    /// Input: (or A (or B C) (or B C))
    /// Expected after simplify: (or A B C)
    #[test]
    fn test_root_or_structural_simplification() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.predicate("A");
        let b = builder.predicate("B");
        let c = builder.predicate("C");

        let inner1 = builder.or(vec![b, c]);
        let inner2 = builder.or(vec![b, c]);

        let root = builder.or(vec![a, inner1, inner2]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 3); // A, B, C
    }

    /// Test simplification of an AND node with a single child after flattening/deduplication.
    ///
    /// Input: (and (and A))
    /// Expected after simplify: A
    #[test]
    fn test_and_single_child_reduction() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.predicate("A");
        let inner = builder.and(vec![a]);
        let root = builder.and(vec![inner]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        // After simplification, root should be replaced by A
        assert_eq!(root_node.kind(), ExprKind::Predicate);
    }

    /// Test simplification of an empty AND node.
    ///
    /// Input: (and)
    /// Expected: neutral value (depends on semantics)
    #[test]
    fn test_empty_and_node() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let root = builder.and(vec![]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        // Depending on semantics, could still be AND
        assert!(root_node.kind() == ExprKind::And);
    }

    /// Test simplification of an empty OR node.
    ///
    /// Input: (or)
    /// Expected: neutral value (depends on semantics)
    #[test]
    fn test_empty_or_node() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let root = builder.or(vec![]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        // Depending on semantics, could still be OR
        assert!(root_node.kind() == ExprKind::Or);
    }

    /// Test that simplify_node correctly dispatches to simplify_not_node
    /// and simplifies a double negation.
    /// Input: (not (not A))
    /// Expected: A
    #[test]
    fn test_simplify_node_double_negation() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.predicate("A");
        let not1 = builder.not(a);
        let root = builder.not(not1);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();
        let input = expr.to_syntax_string(&interner);

        simplify_node(expr.root_id().unwrap(), &mut expr).unwrap();

        let output = expr.to_syntax_string(&interner);
        print!("{} -> {}", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Predicate);
        assert_eq!(output, "A");
    }

    /// Test that simplify_node does NOT simplify (not (and)).
    /// Input: (not (and))
    /// Expected: unchanged
    #[test]
    fn test_simplify_node_not_over_empty_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_and = builder.and(vec![]);
        let root = builder.not(empty_and);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();
        let input = expr.to_syntax_string(&interner);

        simplify_node(expr.root_id().unwrap(), &mut expr).unwrap();

        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        // Root must still be NOT
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);

        // Expression unchanged
        assert_eq!(output, "(or)");
    }

    /// Test that simplify_node correctly simplifies a double negation
    /// around an AND subtree.
    /// Input: (not (not (and A B)))
    /// Expected: (and A B)
    #[test]
    fn test_simplify_node_double_negation_on_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.predicate("A");
        let b = builder.predicate("B");
        let and_ab = builder.and(vec![a, b]);
        let not_inner = builder.not(and_ab);
        let root = builder.not(not_inner);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();
        let input = expr.to_syntax_string(&interner);

        simplify_node(expr.root_id().unwrap(), &mut expr).unwrap();

        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);
        assert_eq!(output, "(and A B)");
    }

}
