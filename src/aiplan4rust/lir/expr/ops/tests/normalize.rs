#[cfg(test)]
mod tests {
    use crate::aiplan4rust::lang::ArithmeticOp;
    use super::*;
    use crate::aiplan4rust::lir::expr::{ExprContent, ExprKind};
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::tree::{Node, SyntaxContent};

    /// Complex nested AND flattening + structural deduplication.
    ///
    /// Input: (and (and A B) (and B C) (and (and A B) D))
    /// Expected: (and (A) (B) (C) (D))
    #[test]
    fn test_complex_nested_and_deduplication() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Using numeric IDs to represent predicates A, B, C, and D.
        // A=1, B=2, C=3, D=4
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);
        let d = builder.atomic_formula(4, vec![]);

        // Constructing the nested structure:
        // (and (and 1 2) (and 2 3) (and (and 1 2) 4))
        let inner1 = builder.and(vec![a, b]);
        let inner2 = builder.and(vec![b, c]);
        let inner3 = builder.and(vec![inner1, d]);

        let root = builder.and(vec![inner1, inner2, inner3]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Run expr (includes flattening and structural deduplication)
        normalize(&mut expr)?;

        // --- VALIDATION ---

        // 1. Root must be an AND node
        let root_id = expr.root_id().expect("Root should exist after expr");
        assert_eq!(expr.get_node_kind(root_id), Some(ExprKind::And));

        // 2. Expected children count: 4 (predicates 1, 2, 3, and 4)
        // The expr must have:
        // - Flattened all nested ANDs
        // - Removed the duplicate of predicate '2'
        // - Removed the duplicate of the subtree '(and 1 2)'
        let root_node = expr.try_node(root_id)?;
        assert_eq!(
            root_node.children().len(),
            4,
            "The expression should have exactly 4 children after flattening and deduplication"
        );

        // 3. Verify that all children are AtomicFormula (leaves of the flattened AND)
        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }

    /// AND with nested ANDs and duplicates.
    ///
    /// Input: (and A (and B C) (and B C))
    /// Expected: (and (A) (B) (C))
    #[test]
    fn test_root_and_structural_simplification() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Mapping predicates to IDs: A=1, B=2, C=3
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        // Constructing sub-trees: inner1 and inner2 are structurally identical (and B C)
        let inner1 = builder.and(vec![b, c]);
        let inner2 = builder.and(vec![b, c]);

        // Constructing the root: (and A (and B C) (and B C))
        let root = builder.and(vec![a, inner1, inner2]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // 1. Flattening: merges inner1 and inner2 into the root.
        // 2. Deduplication: removes the duplicate results of B and C.
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // The root must remain an AND node
        assert_eq!(root_node.kind(), ExprKind::And);

        // After flattening and deduplication, we expect exactly 3 children: A, B, and C.
        // Input: (and 1 (and 2 3) (and 2 3))
        // Flattened: (and 1 2 3 2 3)
        // Deduplicated: (and 1 2 3)
        assert_eq!(
            root_node.children().len(),
            3,
            "Root should have exactly 3 children after flattening and deduplication"
        );

        // Ensure all resulting children are AtomicFormula (the leaf atoms)
        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }

    /// OR with nested ORs and duplicates.
    ///
    /// Input: (or A (or B C) (or B C))
    /// Expected: (or (A) (B) (C))
    #[test]
    fn test_root_or_structural_simplification() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Mapping predicates to IDs: A=1, B=2, C=3
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        // Creating two structurally identical sub-trees: (or B C)
        let inner1 = builder.or(vec![b, c]);
        let inner2 = builder.or(vec![b, c]);

        // Initial structure: (or A (or B C) (or B C))
        let root = builder.or(vec![a, inner1, inner2]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // 1. Flattening: Merges nested OR nodes into the root OR.
        // 2. Deduplication: Removes identical child nodes (B and C).
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // The root must be an OR node
        assert_eq!(root_node.kind(), ExprKind::Or);

        // After expr, we expect exactly 3 unique children: A, B, and C.
        // Input: (or 1 (or 2 3) (or 2 3))
        // Processed: (or 1 2 3)
        assert_eq!(
            root_node.children().len(),
            3,
            "Root should have exactly 3 children after flattening and deduplication"
        );

        // Verify that every child is an AtomicFormula
        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }

    /// Test structural deduplication in a root OR node with duplicate subtrees, verifying order-independence.
    ///
    /// Input: (or (or (A) (B)) (or (B) (A)) (C))
    /// Expected: (or (A) (B) (C))
    #[test]
    fn test_root_or_structural_duplicates_order_independent() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Mapping predicates to IDs: A=1, B=2, C=3
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        // Constructing sub-trees with different child orders:
        // inner1: (or 1 2)
        // inner2: (or 2 1)
        let inner1 = builder.or(vec![a, b]);
        let inner2 = builder.or(vec![b, a]);

        // Root structure: (or (or 1 2) (or 2 1) 3)
        let root = builder.or(vec![inner1, inner2, c]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // The pass should flatten both inner ORs and then realize
        // that the resulting sequence [1, 2, 2, 1, 3] contains duplicates.
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root missing");
        let root_node = expr.try_node(root_id)?;

        assert_eq!(root_node.kind(), ExprKind::Or);

        // After flattening and deduplication, we expect exactly 3 children: A, B, and C.
        // Order in the final vector doesn't matter for ops, but the count must be 3.
        assert_eq!(
            root_node.children().len(),
            3,
            "Should have exactly 3 children after order-independent deduplication"
        );

        // Verify all children are atoms
        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }

    /// AND with a single child after flattening.
    ///
    /// Input: (and (and A))
    /// Expected: (A)
    #[test]
    fn test_and_single_child_reduction() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Mapping predicate A to ID 1
        let a = builder.atomic_formula(1, vec![]);

        // Creating a nested structure with single children: (and (and 1))
        let inner = builder.and(vec![a]);
        let root = builder.and(vec![inner]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // This should collapse both AND nodes since they only have one child,
        // leaving only the AtomicFormula (1).
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");

        // The root should no longer be an AND node.
        // It should have been reduced directly to the AtomicFormula.
        assert_eq!(
            expr.get_node_kind(root_id),
            Some(ExprKind::AtomicFormula),
            "The single-child AND chain should be reduced to the leaf atom"
        );

        // Verify the content of the leaf is still our predicate 1
        let root_node = expr.try_node(root_id)?;
        let pred_leaf_id = root_node.children()[0];
        let pred_leaf = expr.try_node(pred_leaf_id)?;

        if let ExprContent::Predicate(pid) = pred_leaf.content() {
            assert_eq!(pid.as_usize(), 1);
        } else {
            panic!("Root's child should be a predicate ID leaf");
        }

        Ok(())
    }

    /// Empty AND.
    ///
    /// Input: (and)
    /// Expected: (and)
    #[test]
    fn test_empty_and_node() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Create an empty AND node: (and)
        let root = builder.and(vec![]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // An empty AND should remain an empty AND (representing logical TRUE).
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // Ensure it is still an AND node
        assert_eq!(root_node.kind(), ExprKind::And);

        // Ensure it remains empty
        assert_eq!(
            root_node.children().len(),
            0,
            "Empty AND node should remain empty (representing True)"
        );

        Ok(())
    }

    /// Empty OR.
    ///
    /// Input: (or)
    /// Expected: (or)
    #[test]
    fn test_empty_or_node() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Create an empty OR node: (or)
        let root = builder.or(vec![]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // An empty OR should remain an empty OR (representing logical FALSE).
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // Ensure it is still an OR node
        assert_eq!(root_node.kind(), ExprKind::Or);

        // Ensure it remains empty
        assert_eq!(
            root_node.children().len(),
            0,
            "Empty OR node should remain empty (representing False)"
        );

        Ok(())
    }

    /// Simplify NOT: double negation.
    ///
    /// Input: (not (not A))
    /// Expected: (A)
    #[test]
    fn test_simplify_node_double_negation() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Mapping predicate A to ID 1
        let a = builder.atomic_formula(1, vec![]);

        // Creating nested NOTs: (not (not (1)))
        let inner_not = builder.not(a);
        let root = builder.not(inner_not);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // This should detect the double negation and strip both NOT nodes,
        // promoting the AtomicFormula to the root.
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist after expr");

        // The root should no longer be a NOT node.
        // It should be the AtomicFormula directly.
        assert_eq!(
            expr.get_node_kind(root_id),
            Some(ExprKind::AtomicFormula),
            "Double negation should be eliminated, leaving only the atom"
        );

        // Verify the leaf content is still predicate 1
        let root_node = expr.try_node(root_id)?;
        let pred_leaf_id = root_node.children()[0];
        let pred_leaf = expr.try_node(pred_leaf_id)?;

        if let ExprContent::Predicate(pid) = pred_leaf.content() {
            assert_eq!(pid.as_usize(), 1);
        } else {
            panic!("Root's child should be the predicate ID leaf");
        }

        Ok(())
    }

    /// Simplify NOT over empty AND.
    ///
    /// Input: (not (and))
    /// Expected: (or)
    #[test]
    fn test_simplify_node_not_over_empty_and() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Create an empty AND (Logical True)
        let empty_and = builder.and(vec![]);

        // Create NOT over empty AND: (not (and))
        let root = builder.not(empty_and);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // (not true) should be simplified to false.
        // In LIR: (not (and)) -> (or)
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // The root should now be an OR node (representing False)
        assert_eq!(
            root_node.kind(),
            ExprKind::Or,
            "Negating an empty AND should result in an empty OR"
        );

        // The OR node should be empty
        assert_eq!(
            root_node.children().len(),
            0,
            "The resulting OR node should have no children (Logical False)"
        );

        Ok(())
    }

    /// Double negation over AND subtree.
    ///
    /// Input: (not (not (and A B)))
    /// Expected: (and (A) (B))
    #[test]
    fn test_simplify_node_double_negation_on_and() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Mapping predicates to IDs: A=1, B=2
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let and_ab = builder.and(vec![a, b]);

        // Constructing (not (not (and 1 2)))
        let inner_not = builder.not(and_ab);
        let root = builder.not(inner_not);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // Both NOT layers should be stripped, leaving the AND node as the root.
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // 1. The root should now be the AND node
        assert_eq!(
            root_node.kind(),
            ExprKind::And,
            "Double negation over AND should result in the AND node being promoted to root"
        );

        // 2. The AND node should still have its 2 original children (A and B)
        assert_eq!(
            root_node.children().len(),
            2,
            "The resulting AND node should preserve its children"
        );

        // 3. Verify the children are the expected AtomicFormulas
        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }


    /// Input: (A -> (B -> C))
    /// Expected output: (or (not (A)) (or (not (B)) (C)))
    #[test]
    fn test_nested_imply_left_to_right() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Mapping predicates to IDs: A=1, B=2, C=3
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        // Constructing nested implication: (imply 1 (imply 2 3))
        let inner_imply = builder.imply(b, c);
        let outer_imply = builder.imply(a, inner_imply);

        builder.set_root(outer_imply)?;
        let mut expr = builder.finish();

        // Apply expr:
        // 1. Convert outer imply: (or (not 1) (imply 2 3))
        // 2. Convert inner imply: (or (not 1) (or (not 2) 3))
        // 3. Flatten ORs: (or (not 1) (not 2) 3)
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // The root should be an OR node
        assert_eq!(root_node.kind(), ExprKind::Or);

        // After flattening, we expect 3 children: (not A), (not B), and C
        assert_eq!(
            root_node.children().len(),
            3,
            "Nested implications should be flattened into a single OR with 3 children"
        );

        // Verify the presence of two NOT nodes and one AtomicFormula
        let mut not_count = 0;
        let mut atom_count = 0;

        for &child_id in root_node.children() {
            match expr.get_node_kind(child_id) {
                Some(ExprKind::Not) => not_count += 1,
                Some(ExprKind::AtomicFormula) => atom_count += 1,
                _ => panic!("Unexpected node kind in normalized implication"),
            }
        }

        assert_eq!(not_count, 2, "Expected two negated antecedents");
        assert_eq!(atom_count, 1, "Expected one positive consequent");

        Ok(())
    }

    /// Input: ((A -> B) -> C)
    /// Expected output: (or (not (or (not (A)) (B))) (C))
    #[test]
    fn test_nested_imply_right_to_left() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Predicate IDs: A=1, B=2, C=3
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        // Inner implication: (imply 1 2) -> (or (not 1) 2)
        let inner_imply = builder.imply(a, b);
        // Outer implication: (imply (imply 1 2) 3)
        let outer_imply = builder.imply(inner_imply, c);

        builder.set_root(outer_imply)?;
        let mut expr = builder.finish();

        // Normalization process:
        // 1. Convert outer imply: (or (not (imply 1 2)) 3)
        // 2. Convert inner imply: (or (not (or (not 1) 2)) 3)
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // The root should be an OR node
        assert_eq!(root_node.kind(), ExprKind::Or);

        // It should have 2 main children: C and the negated inner OR
        assert_eq!(
            root_node.children().len(),
            2,
            "The outer OR should have exactly two children (the consequent and the negated antecedent)"
        );

        // Verify the presence of the NOT node (negated antecedent)
        let has_not = root_node.children().iter().any(|&id|
            expr.get_node_kind(id) == Some(ExprKind::Not)
        );
        assert!(has_not, "One child must be a NOT node containing the transformed inner implication");

        Ok(())
    }

    /// Nested addition and multiplication:
    ///
    /// Input: (+ 1 (* 2 3) 4)
    /// Expected: 11
    #[test]
    fn test_add_mul_nested() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Numeric constants
        let one = builder.number(1.0);
        let two = builder.number(2.0);
        let three = builder.number(3.0);
        let four = builder.number(4.0);

        // Constructing the expression: (+ 1 (* 2 3) 4)
        let mul = builder.mul(vec![two, three]);
        let root = builder.add(vec![one, mul, four]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // 1. Evaluate (* 2 3) -> 6
        // 2. Evaluate (+ 1 6 4) -> 11
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // The root should now be a single Number node (constant folding)
        assert_eq!(
            root_node.kind(),
            ExprKind::Number,
            "Nested arithmetic constants should be folded into a single Number"
        );

        // Verify the value is exactly 11.0
        if let ExprContent::Float(val) = root_node.content() {
            assert_eq!(val.into_inner(), 11.0);
        } else {
            panic!("Root content should be a floating point number");
        }

        Ok(())
    }
    /// Nested division and subtraction:
    ///
    /// Input: (- (/ 20 2) 3)
    /// Expected: 7
    #[test]
    fn test_div_sub_nested() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Numeric constants
        let twenty = builder.number(20.0);
        let two = builder.number(2.0);
        let three = builder.number(3.0);

        // Constructing the expression: (- (/ 20 2) 3)
        let div = builder.div(vec![twenty, two]);
        let root = builder.sub(vec![div, three]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // 1. Evaluate (/ 20 2) -> 10.0
        // 2. Evaluate (- 10 3) -> 7.0
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // Check that the tree collapsed into a single Number node
        assert_eq!(root_node.kind(), ExprKind::Number);

        // Verify the constant value
        if let ExprContent::Float(val) = root_node.content() {
            assert_eq!(val.into_inner(), 7.0, "The result of (- (/ 20 2) 3) should be 7");
        } else {
            panic!("Root content should be a floating point number");
        }

        Ok(())
    }

    /// Deeply nested operations: (+ (* 2 3) (- 10 4) (/ 20 5))
    ///
    /// Input: (+ (* 2 3) (- 10 4) (/ 20 5))
    /// Expected: 6 + 6 + 4 = 16
    #[test]
    fn test_deeply_nested_operations() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Numeric constants
        let two = builder.number(2.0);
        let three = builder.number(3.0);
        let ten = builder.number(10.0);
        let four = builder.number(4.0);
        let twenty = builder.number(20.0);
        let five = builder.number(5.0);

        // Branch 1: (* 2 3) = 6
        let mul = builder.mul(vec![two, three]);
        // Branch 2: (- 10 4) = 6
        let sub = builder.sub(vec![ten, four]);
        // Branch 3: (/ 20 5) = 4
        let div = builder.div(vec![twenty, five]);

        // Root: (+ 6 6 4) = 16
        let root = builder.add(vec![mul, sub, div]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // Every arithmetic branch should be folded recursively.
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // Ensure the entire tree collapsed into a single Number
        assert_eq!(root_node.kind(), ExprKind::Number);

        // Verify the final calculated value
        if let ExprContent::Float(val) = root_node.content() {
            assert_eq!(
                val.into_inner(),
                16.0,
                "The expression (+ (* 2 3) (- 10 4) (/ 20 5)) should fold to 16"
            );
        } else {
            panic!("Root content should be a floating point number");
        }

        Ok(())
    }

    /// Nested operation with non-constant child should remain unchanged:
    ///
    /// Input: (+ 2 (* A 3))
    /// Expected: (+ 2 (* A 3))  (cannot simplification because A is variable)
    #[test]
    fn test_nested_with_variable_child() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Numeric constants
        let two = builder.number(2.0);
        let three = builder.number(3.0);

        // An atomic formula (variable/predicate) that cannot be folded
        let a = builder.atomic_formula(1, vec![]);

        // Constructing the expression: (+ 2 (* A 3))
        let mul = builder.mul(vec![a, three]);
        let root = builder.add(vec![two, mul]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // Apply expr:
        // "reduce" will now perform partial reduction on (+ 2 (...)) and (* A 3)
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // 1. Verify the root is still an Addition operation
        assert_eq!(root_node.kind(), ExprKind::Operation);
        assert!(matches!(root_node.content().as_arithmetic_op(), Some(ArithmeticOp::Add)));

        // 2. Verify structure: Addition should have the constant '2' and the 'Mul' node
        let children = root_node.children();
        assert_eq!(children.len(), 2, "Addition should still have 2 children");

        // Find the multiplication child (its ID might have changed or its position swapped)
        let mul_child_id = children.iter()
            .find(|&&id| expr.get_node_kind(id) == Some(ExprKind::Operation))
            .expect("Multiplication node should still exist under the addition");

        let mul_node = expr.try_node(*mul_child_id)?;
        assert!(matches!(mul_node.content().as_arithmetic_op(), Some(ArithmeticOp::Mul)));

        // 3. Verify the multiplication still contains 'A' and '3.0'
        let mul_children = mul_node.children();
        assert_eq!(mul_children.len(), 2, "Multiplication should still have 2 children");

        let mut found_a = false;
        let mut found_three = false;

        for &c_id in mul_children {
            let c_node = expr.try_node(c_id)?;
            match c_node.kind() {
                ExprKind::AtomicFormula => found_a = true,
                ExprKind::Number => {
                    if let Some(val) = c_node.content().as_float() {
                        if val.0 == 3.0 { found_three = true; }
                    }
                }
                _ => (),
            }
        }

        assert!(found_a, "Variable 'A' not found in multiplication");
        assert!(found_three, "Constant '3.0' not found in multiplication");

        Ok(())
    }

    /// Simple implication transformation:
    ///
    /// Input: (imply A B)
    /// Expected: (or (not A) B)
    #[test]
    fn test_normalize_simple_imply() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Mapping predicates to IDs: A=1, B=2
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);

        // Creating the implication: (imply 1 2)
        let imply = builder.imply(a, b);

        builder.set_root(imply)?;
        let mut expr = builder.finish();

        // Apply expr:
        // The implication must be rewritten as a disjunction (OR).
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // 1. The root should now be an OR node
        assert_eq!(
            root_node.kind(),
            ExprKind::Or,
            "Implication should be transformed into an OR node"
        );

        // 2. It should have exactly two children: (not A) and B
        assert_eq!(root_node.children().len(), 2);

        // 3. Verify the structure of the children.
        // Note: The order in the children vector depends on your implementation,
        // but usually it's [negated_antecedent, consequent].
        let mut has_not = false;
        let mut has_atom = false;

        for &child_id in root_node.children() {
            match expr.get_node_kind(child_id) {
                Some(ExprKind::Not) => has_not = true,
                Some(ExprKind::AtomicFormula) => has_atom = true,
                _ => {}
            }
        }

        assert!(has_not, "Missing negated antecedent (not A)");
        assert!(has_atom, "Missing consequent (B)");

        Ok(())
    }

    /// Combination of implication and double negation:
    ///
    /// Input: (imply (not (not A)) B)
    /// Process: (or (not (not (not A))) B) -> (or (not A) B)
    /// Expected: (or (not A) B)
    #[test]
    fn test_normalize_with_double_negation() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Mapping predicates to IDs: A=1, B=2
        let a = builder.atomic_formula(1, vec![]);
        let not_a = builder.not(a);
        let double_not_a = builder.not(not_a);
        let b = builder.atomic_formula(2, vec![]);

        // Construction: (imply (not (not A)) B)
        let imply = builder.imply(double_not_a, b);

        builder.set_root(imply)?;
        let mut expr = builder.finish();

        // Normalisation :
        // 1. L'implication devient (or (not (not (not A))) B)
        // 2. La triple négation est réduite à une simple négation (not A)
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // Vérification de la racine OR
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 2);

        // Vérification que le résultat est bien (or (not A) B)
        // On s'attend à ce que l'un des enfants soit un NOT et l'autre un AtomicFormula
        let mut has_not_a = false;
        let mut has_b = false;

        for &child_id in root_node.children() {
            let child = expr.try_node(child_id)?;
            match child.kind() {
                ExprKind::Not => {
                    // Le fils du NOT doit être A (ID 1)
                    let grand_child_id = child.children()[0];
                    if let Some(ExprKind::AtomicFormula) = expr.get_node_kind(grand_child_id) {
                        has_not_a = true;
                    }
                }
                ExprKind::AtomicFormula => {
                    has_b = true;
                }
                _ => {}
            }
        }

        assert!(has_not_a, "Should have simplified to (not A)");
        assert!(has_b, "Should have kept B as the second disjunct");

        Ok(())
    }

    /// Implication with a complex consequent:
    ///
    /// Input: (imply A (and B C))
    /// Expected: (or (not A) (and B C))
    #[test]
    fn test_normalize_with_and_or_nodes() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // Mapping predicates to IDs: A=1, B=2, C=3
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        // (and B C)
        let and_bc = builder.and(vec![b, c]);
        // (imply A (and B C))
        let imply = builder.imply(a, and_bc);

        builder.set_root(imply)?;
        let mut expr = builder.finish();

        // Normalization:
        // The implication is converted to OR, but the inner AND should be preserved.
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // 1. Verify root is OR
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 2);

        // 2. Check children: one should be (not A), the other should be (and B C)
        let mut has_negated_a = false;
        let mut has_and_bc = false;

        for &child_id in root_node.children() {
            let child = expr.try_node(child_id)?;
            match child.kind() {
                ExprKind::Not => {
                    let grandchild_id = child.children()[0];
                    if expr.get_node_kind(grandchild_id) == Some(ExprKind::AtomicFormula) {
                        has_negated_a = true;
                    }
                }
                ExprKind::And => {
                    if child.children().len() == 2 {
                        has_and_bc = true;
                    }
                }
                _ => {}
            }
        }

        assert!(has_negated_a, "Missing negated antecedent (not A)");
        assert!(has_and_bc, "Missing original consequent structure (and B C)");

        Ok(())
    }

    /// Normalization with nested quantifiers using typed variables:
    ///
    /// Input: (imply (forall (?X - T1) (A)) (exists (?Y - T2) (B)))
    /// Expected Output: (or (not (forall (?X - T1) (A))) (exists (?Y - T2) (B)))
    #[test]
    fn test_normalize_with_quantifiers() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Prepare variables and lists
        let var_x = builder.typed_variable(1, &[101]); // ?X - T1
        let list_x = builder.typed_variable_list(vec![var_x]);

        let var_y = builder.typed_variable(2, &[102]); // ?Y - T2
        let list_y = builder.typed_variable_list(vec![var_y]);

        // 2. Build atomic formulas
        let atomic_a = builder.atomic_formula(3, vec![]);
        let atomic_b = builder.atomic_formula(4, vec![]);

        // 3. Build quantifiers
        let forall_node = builder.forall(list_x, atomic_a);
        let exists_node = builder.exists(list_y, atomic_b);

        // 4. Build implication: (imply (forall...) (exists...))
        let imply = builder.imply(forall_node, exists_node);
        builder.set_root(imply)?;
        let mut expr = builder.finish();

        // 5. Normalization
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // Verify the root is an OR node
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 2);

        // Verify structure: (not (forall...)) and (exists...)
        let mut has_not_forall = false;
        let mut has_exists = false;

        for &child_id in root_node.children() {
            let child = expr.try_node(child_id)?;
            match child.kind() {
                ExprKind::Not => {
                    let grandchild_id = child.children()[0];
                    if expr.get_node_kind(grandchild_id) == Some(ExprKind::Forall) {
                        has_not_forall = true;
                    }
                }
                ExprKind::Exists => {
                    has_exists = true;
                }
                _ => {}
            }
        }

        assert!(has_not_forall, "Antecedent (forall) should be wrapped in a NOT node");
        assert!(has_exists, "Consequent (exists) should be preserved");

        Ok(())
    }

    /// Test: When with empty AND condition and non-trivial effect
    ///
    /// Input: (when (and) (and A B C))
    /// Process: (when True Effect) -> Effect
    /// Expected Output: (and A B C)
    #[test]
    fn test_when_empty_and_complex_effect() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Prepare atomic formulas: A=1, B=2, C=3
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        // 2. Build the effect: (and A B C)
        let effect = builder.and(vec![a, b, c]);

        // 3. Build the condition: (and) -> True
        let condition = builder.and(vec![]);

        // 4. Build the WHEN node
        let when_node = builder.when(condition, effect);

        builder.set_root(when_node)?;
        let mut expr = builder.finish();

        // 5. Normalization:
        // The normalizer should detect that the condition is always true
        // and replace the WHEN node directly with its effect.
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // The root should now be the AND node of the effect
        assert_eq!(
            root_node.kind(),
            ExprKind::And,
            "The WHEN node should be simplified to its effect if the condition is True"
        );

        // Verify that the 3 children (A, B, C) are present
        let children = root_node.children();
        assert_eq!(children.len(), 3);

        for &child_id in children {
            let child_node = expr.try_node(child_id)?;
            assert_eq!(child_node.kind(), ExprKind::AtomicFormula);
        }

        Ok(())
    }

    /// Test: When with empty OR condition and multiple AND effect
    ///
    /// Input: (when (or) (and X Y Z))
    /// Process: (when False Effect) -> True (Empty And)
    /// Expected Output: (and)
    #[test]
    fn test_when_empty_or_complex_effect() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Prepare atomic formulas: X=1, Y=2, Z=3
        let x = builder.atomic_formula(1, vec![]);
        let y = builder.atomic_formula(2, vec![]);
        let z = builder.atomic_formula(3, vec![]);

        // 2. Build the effect: (and X Y Z)
        let effect = builder.and(vec![x, y, z]);

        // 3. Build the condition: (or) -> False
        let condition = builder.or(vec![]);

        // 4. Build the WHEN node
        let when_node = builder.when(condition, effect);

        builder.set_root(when_node)?;
        let mut expr = builder.finish();

        // 5. Normalization:
        // Since (or) is False, the conditional effect can never trigger.
        // It must be replaced by an empty effect (and).
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // The root must be an empty AND
        assert_eq!(
            root_node.kind(),
            ExprKind::And,
            "A conditional effect with a False condition must become an empty (and)"
        );

        assert_eq!(
            root_node.children().len(),
            0,
            "The effect should have been totally removed"
        );

        Ok(())
    }

    /// Test: When with condition equal to complex effect
    ///
    /// Input: (when (and A B) (and A B))
    /// Process: If the condition implies the effect (and both are atoms/conjunctions),
    ///          the effect is redundant.
    /// Expected Output: (and)
    #[test]
    fn test_when_condition_equal_effect() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Prepare atomic formulas: A=1, B=2
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);

        // 2. Build the condition: (and A B)
        let condition = builder.and(vec![a, b]);

        // 3. Build the effect: (and A B)
        // Note: Recreating the structures ensures the normalizer
        // compares content rather than just pointer addresses.
        let a2 = builder.atomic_formula(1, vec![]);
        let b2 = builder.atomic_formula(2, vec![]);
        let effect = builder.and(vec![a2, b2]);

        // 4. Build the WHEN node
        let when_node = builder.when(condition, effect);

        builder.set_root(when_node)?;
        let mut expr = builder.finish();

        // 5. Normalization:
        // The effect (and A B) is already satisfied if the condition (and A B) is true.
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // The root must be an empty AND (null effect)
        assert_eq!(
            root_node.kind(),
            ExprKind::And,
            "An effect identical to its condition is redundant and should be simplified"
        );

        assert_eq!(
            root_node.children().len(),
            0,
            "The redundant effect should have been removed"
        );

        Ok(())
    }

    /// Test: When with non-trivial condition and empty AND effect
    ///
    /// Input: (when (and A B) (and))
    /// Process: A conditional effect that does nothing is itself a no-op.
    /// Expected Output: (and)
    #[test]
    fn test_when_nontrivial_condition_empty_effect() -> Result<(), LogicError> {
        let mut builder = ExprBuilder::new();

        // 1. Prepare the complex condition: (and A B)
        let cond_a = builder.atomic_formula(1, vec![]);
        let cond_b = builder.atomic_formula(2, vec![]);
        let condition = builder.and(vec![cond_a, cond_b]);

        // 2. Prepare the empty effect: (and)
        let empty_and = builder.and(vec![]);

        // 3. Build the WHEN node: (when (and A B) (and))
        let when_node = builder.when(condition, empty_and);

        builder.set_root(when_node)?;
        let mut expr = builder.finish();

        // 4. Normalization:
        // Since the effect is empty, the condition no longer needs to be evaluated.
        // The WHEN node should be simplified into a simple empty (and).
        normalize(&mut expr)?;

        // --- VALIDATION ---

        let root_id = expr.root_id().expect("Root should exist");
        let root_node = expr.try_node(root_id)?;

        // The root must be an empty AND
        assert_eq!(
            root_node.kind(),
            ExprKind::And,
            "A conditional effect with no effect should be removed"
        );

        assert_eq!(
            root_node.children().len(),
            0,
            "The tree should contain no children (empty effect)"
        );

        Ok(())
    }
}
