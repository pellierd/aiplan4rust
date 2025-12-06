use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind};
use crate::aiplan4rust::lir::expr::normalizer::{and_or, arithmetic, assign, comparison, not, quantifier, when};
use crate::aiplan4rust::lir::expr::transform::eliminate_imply::eliminate_imply;
use crate::aiplan4rust::lir::expr::transform::push_negation::push_negation;
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
pub fn normalize(expr: &mut Expr) -> Result<(), ExprError> {
    let Some(root_id) = expr.root_id() else { return Ok(()); };

    pre_process(expr)?;

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
        normalize_node(node_id, expr)?;
    }

    /// TO ADD
    /// Factorise les parties commpostunes des expressions.
    /// Exemple: `(A ∧ B) ∨ (A ∧ C) -> A ∧ (B ∨ C)`.
    Ok(())
}

fn pre_process(expr: &mut Expr) -> Result<(), ExprError> {
    let Some(root_id) = expr.root_id() else { return Ok(()); };
    eliminate_imply(root_id, expr)?;
    push_negation(root_id, expr)

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
pub(crate) fn normalize_node(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {

    let kind = expr.try_node(node_id)?.kind();

    match kind {
        ExprKind::And | ExprKind::Or => {
            and_or::normalize(node_id, expr)?;
        }
        ExprKind::Not => {
            not::normalize(node_id, expr)?;
        }
        ExprKind::Forall | ExprKind::Exists => {
            quantifier::normalize(node_id, expr)?;
        }
        ExprKind::Imply => {
            return Err(ExprError::UnexpectedNodeKind {
                node_id,
                kind: ExprKind::Imply,
            });
        }
        ExprKind::Assign => {
            assign::normalize(node_id, expr)?;
        }
        ExprKind::FComp => {
            comparison::normalize(node_id, expr)?;
        }
        ExprKind::Operation => {
            arithmetic::normalize(node_id, expr)?;
        }
        ExprKind::When => {
            when::normalize(node_id, expr)?;
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

    /// Complex nested AND flattening + structural deduplication.
    ///
    /// Input: (and (and A B) (and B C) (and (and A B) D))
    /// Expected: (and (A) (B) (C) (D))
    #[test]
    fn test_complex_nested_and_deduplication() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let d = builder.atomic_formula("D", vec![]);

        let inner1 = builder.and(vec![a, b]);
        let inner2 = builder.and(vec![b, c]);
        let inner3 = builder.and(vec![inner1, d]);

        let root = builder.and(vec![inner1, inner2, inner3]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "(and (A) (B) (C) (D))");
    }

    /// AND with nested ANDs and duplicates.
    ///
    /// Input: (and A (and B C) (and B C))
    /// Expected: (and (A) (B) (C))
    #[test]
    fn test_root_and_structural_simplification() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);

        let inner1 = builder.and(vec![b, c]);
        let inner2 = builder.and(vec![b, c]);

        let root = builder.and(vec![a, inner1, inner2]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "(and (A) (B) (C))");
    }

    /// OR with nested ORs and duplicates.
    ///
    /// Input: (or A (or B C) (or B C))
    /// Expected: (or (A) (B) (C))
    #[test]
    fn test_root_or_structural_simplification() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);

        let inner1 = builder.or(vec![b, c]);
        let inner2 = builder.or(vec![b, c]);

        let root = builder.or(vec![a, inner1, inner2]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "(or (A) (B) (C))");
    }

    /// Test structural deduplication in a root OR node with duplicate subtrees, verifying order-independence.
    ///
    /// Input: (or (or (A) (B)) (or (B) (A)) (C))
    /// Expected: (or (A) (B) (C))
    #[test]
    fn test_root_or_structural_duplicates_order_independent() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);

        let inner1 = builder.or(vec![a, b]);
        let inner2 = builder.or(vec![b, a]); // reversed order
        let c = builder.atomic_formula("C", vec![]);
        let root = builder.or(vec![inner1, inner2, c]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 3);
        assert_eq!(output, "(or (A) (B) (C))");
    }

    /// AND with a single child after flattening.
    ///
    /// Input: (and (and A))
    /// Expected: (A)
    #[test]
    fn test_and_single_child_reduction() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let inner = builder.and(vec![a]);
        let root = builder.and(vec![inner]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "(A)");
    }

    /// Empty AND.
    ///
    /// Input: (and)
    /// Expected: (and)
    #[test]
    fn test_empty_and_node() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let root = builder.and(vec![]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "(and)");
    }

    /// Empty OR.
    ///
    /// Input: (or)
    /// Expected: (or)
    #[test]
    fn test_empty_or_node() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let root = builder.or(vec![]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "(or)");
    }

    /// Simplify NOT: double negation.
    ///
    /// Input: (not (not A))
    /// Expected: (A)
    #[test]
    fn test_simplify_node_double_negation() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let inner_not = builder.not(a);
        let root = builder.not(inner_not);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "(A)");
    }

    /// Simplify NOT over empty AND.
    ///
    /// Input: (not (and))
    /// Expected: (or)
    #[test]
    fn test_simplify_node_not_over_empty_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);
        let empty_and = builder.and(vec![]);
        let root = builder.not(empty_and);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or)");
    }

    /// Double negation over AND subtree.
    ///
    /// Input: (not (not (and A B)))
    /// Expected: (and (A) (B))
    #[test]
    fn test_simplify_node_double_negation_on_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let and_ab = builder.and(vec![a, b]);

        let inner_not = builder.not(and_ab);
        let root = builder.not(inner_not);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "(and (A) (B))");
    }


    /// Input: (A -> (B -> C))
    /// Expected output: (or (not (A)) (or (not (B)) (C)))
    #[test]
    fn test_nested_imply_left_to_right() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let inner_imply = builder.imply(b, c);
        let outer_imply = builder.imply(a, inner_imply);

        builder.set_root(outer_imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (C) (not (B)) (not (A)))");
    }

    /// Input: ((A -> B) -> C)
    /// Expected output: (or (not (or (not (A)) (B))) (C))
    #[test]
    fn test_nested_imply_right_to_left() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let inner_imply = builder.imply(a, b);
        let outer_imply = builder.imply(inner_imply, c);

        builder.set_root(outer_imply).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (C) (not (or (B) (not (A)))))");
    }

    /// Nested addition and multiplication:
    ///
    /// Input: (+ 1 (* 2 3) 4)
    /// Expected: 11
    #[test]
    fn test_add_mul_nested() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let one = builder.number(1.0);
        let two = builder.number(2.0);
        let three = builder.number(3.0);
        let four = builder.number(4.0);

        let mul = builder.mul(vec![two, three]);
        let root = builder.add(vec![one, mul, four]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "11");
    }

    /// Nested division and subtraction:
    ///
    /// Input: (- (/ 20 2) 3)
    /// Expected: 7
    #[test]
    fn test_div_sub_nested() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let twenty = builder.number(20.0);
        let two = builder.number(2.0);
        let three = builder.number(3.0);

        let div = builder.div(vec![twenty, two]);
        let root = builder.sub(vec![div, three]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "7");
    }

    /// Deeply nested operations: (+ (* 2 3) (- 10 4) (/ 20 5))
    ///
    /// Input: (+ (* 2 3) (- 10 4) (/ 20 5))
    /// Expected: 6 + 6 + 4 = 16
    #[test]
    fn test_deeply_nested_operations() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let two = builder.number(2.0);
        let three = builder.number(3.0);
        let ten = builder.number(10.0);
        let four = builder.number(4.0);
        let twenty = builder.number(20.0);
        let five = builder.number(5.0);

        let mul = builder.mul(vec![two, three]); // 6
        let sub = builder.sub(vec![ten, four]);   // 6
        let div = builder.div(vec![twenty, five]); // 4

        let root = builder.add(vec![mul, sub, div]); // 16

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "16");
    }

    /// Nested operation with non-constant child should remain unchanged:
    ///
    /// Input: (+ 2 (* A 3))
    /// Expected: (+ 2 (* A 3))  (cannot simplify because A is variable)
    #[test]
    fn test_nested_with_variable_child() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let two = builder.number(2.0);
        let three = builder.number(3.0);
        let a = builder.atomic_formula("A", vec![]);

        let mul = builder.mul(vec![a, three]);
        let root = builder.add(vec![two, mul]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "(+ 2 (* (A) 3))");
    }

    #[test]
    fn test_normalize_simple_imply() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let imply = builder.imply(a, b);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (B) (not (A)))");
    }

    #[test]
    fn test_normalize_with_double_negation() {
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
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (B) (not (A)))");
    }

    #[test]
    fn test_normalize_with_and_or_nodes() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let and_bc = builder.and(vec![b, c]);
        let imply = builder.imply(a, and_bc);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        normalize(&mut expr).unwrap();
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        let output = expr.to_syntax_string(&interner);
        assert_eq!(output, "(or (and (B) (C)) (not (A)))");
    }

    #[test]
    fn test_normalize_with_quantifiers() {
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

        normalize(&mut expr).unwrap();
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        let output = expr.to_syntax_string(&interner);
        assert_eq!(output, "(or (exists (?Y) (B)) (not (forall (?X) (A))))");
    }

    /// Test 1: When with empty AND condition and non-trivial effect
    /// Input: (when (and) (and A B C))
    /// Expected Output: (and A B C)
    #[test]
    fn test_when_empty_and_complex_effect() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let effect = builder.and(vec![a, b, c]);
        let condition = builder.and(vec![]);
        let when_node = builder.when(condition, effect);

        builder.set_root(when_node).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner); // "(when (and) (and A B C))"
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner); // "(and A B C)"

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        let children = root_node.children();
        assert_eq!(root_node.children().len(), 3);
        // Verify that all three children are AtomicFormula
        for &child_id in children {
            let child_node = expr.try_node(child_id).unwrap();
            assert_eq!(child_node.kind(), ExprKind::AtomicFormula);
        }
    }

    /// Test 2: When with empty OR condition and multiple AND effect
    /// Input: (when (or) (and X Y Z))
    /// Expected Output: (and)
    #[test]
    fn test_when_empty_or_complex_effect() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let x = builder.atomic_formula("X", vec![]);
        let y = builder.atomic_formula("Y", vec![]);
        let z = builder.atomic_formula("Z", vec![]);
        let effect = builder.and(vec![x.clone(), y.clone(), z.clone()]);
        let condition = builder.or(vec![]);
        let when_node = builder.when(condition, effect);

        builder.set_root(when_node).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner); // "(when (or) (and X Y Z))"
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner); // "(and)"

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 0); // fully simplified
    }

    /// Test 3: When with condition equal to complex effect
    /// Input: (when (and A B) (and A B))
    /// Expected Output: (and)
    #[test]
    fn test_when_condition_equal_effect() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let condition = builder.and(vec![a, b]);
        let effect = builder.and(vec![a, b]);
        let when_node = builder.when(condition, effect);

        builder.set_root(when_node).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner); // "(when (and A B) (and A B))"
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner); // "(and)"

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 0); // fully simplified
    }

    /// Test 4: When with non-trivial condition and empty AND effect
    /// Input: (when (and A B) (and))
    /// Expected Output: (and)
    #[test]
    fn test_when_nontrivial_condition_empty_effect() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let cond_a = builder.atomic_formula("A", vec![]);
        let cond_b = builder.atomic_formula("B", vec![]);
        let condition = builder.and(vec![cond_a, cond_b]);
        let empty_and = builder.and(vec![]);
        let when_node = builder.when(condition, empty_and);

        builder.set_root(when_node).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner); // "(when (and A B) (and))"
        normalize(&mut expr).unwrap();
        let output = expr.to_syntax_string(&interner); // "(and)"

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 0);
    }
}
