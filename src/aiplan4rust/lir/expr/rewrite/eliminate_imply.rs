use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::tree::NodeId;

/// Removes all `Imply` nodes in the subtree rooted at `node_id` by transforming
/// each `A -> B` into `(Or(Not(A), B))`.
///
/// This function is the first step in the normalization pipeline. It ensures that
/// implications are eliminated so that subsequent transformations (pushing negations,
/// temporal factorization, simplification) can operate on a simpler logical structure.
///
/// # Behavior
/// - Traverses the subtree in **post-order** (DFS) to ensure children are processed before their parent.
/// - Each `Imply` node encountered is replaced by an `Or` node:
///   - The premise `A` becomes a `Not(A)` node.
///   - The consequence `B` is kept as-is.
///   - The current `Imply` node is converted into `Or(Not(A), B)`.
/// - Non-`Imply` nodes are left unchanged.
///
/// # Preconditions
/// - None; this function can be called on any expression tree.
/// - It should be the **first step** in the normalization pipeline, before `push_negation` and
///   `push_time_specifier`.
///
/// # Dependencies
/// - Independent function; does not require any preprocessing.
/// - Subsequent steps (`push_negation`, `push_time_specifier`, `factorize_time_specifier`) depend
///   on this function having been applied first.
///
/// # Parameters
/// - `node_id`: The root `NodeId` of the subtree to process.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if all `Imply` nodes were successfully removed.
/// - `Err(ExprError)` if accessing or mutating nodes fails.
///
/// # Panics (in debug mode)
/// - Panics if any `Imply` node does not have exactly two children.
///   This is enforced with `debug_assert!`.
///
/// # Notes
/// - This function removes **all** `Imply` nodes in the given subtree, not just a single node.
/// - Further simplifications (like flattening `Or` or handling double negations) should
///   be applied separately if desired.
/// - The function is safe to call independently, but in practice it is used as the first step
///   of the normalization workflow orchestrated by the `normalize` module.
///
/// # Example
/// ```ignore
/// // Suppose `expr` contains multiple Imply nodes in a subtree rooted at node_id
/// eliminate_imply(node_id, &mut expr)?;
/// // All Imply nodes in that subtree are now replaced by Or(Not(premise), consequence)
/// ```
pub fn eliminate_imply(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    // Stack for DFS post-order: (node_id, visited_flag)
    let mut stack = vec![(node_id, false)];

    while let Some((curr_id, visited)) = stack.pop() {
        if visited {
            // Post-order: process the node after its children
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

            // Create Not(premise)
            let not_node = ExprNode::new(ExprKind::Not, ExprContent::None, None);
            let not_premise_id = expr.alloc_with_children(not_node, vec![premise]);

            // Convert current node into Or(Not(premise), consequence)
            let node_mut = expr.try_node_mut(curr_id)?;
            node_mut.set_kind(ExprKind::Or);
            node_mut.set_children(vec![not_premise_id, consequence]);
        } else {
            // Mark as visited and push children
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
    use crate::aiplan4rust::arena::ArenaNode;
    use super::*;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::renderers::LiftedSyntaxDisplay;
    use crate::aiplan4rust::syntax::SyntaxInternerDisplay;

    /// Input: (A -> B)
    /// Expected output: (or (B) (not (A)))
    #[test]
    fn test_nested_imply() {
        let mut builder = ExprBuilder::new();

        // Setup: (imply (1) (imply (2) (3)))
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        let inner_imply = builder.imply(b, c);
        let root_imply = builder.imply(a, inner_imply);

        builder.set_root(root_imply).unwrap();
        let mut expr = builder.finish();

        eliminate_imply(expr.root_id().unwrap(), &mut expr).unwrap();

        // --- Structure Validation ---
        let root_id = expr.root_id().unwrap();
        let root_node = expr.try_node(root_id).unwrap();

        // 1. Root should be OR
        assert_eq!(root_node.kind(), ExprKind::Or);
        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // 2. First child: (not A)
        let not_a_id = children[0];
        assert_eq!(expr.try_node(not_a_id).unwrap().kind(), ExprKind::Not);

        // 3. Second child: (or (not B) C)
        let inner_or_id = children[1];
        let inner_or_node = expr.try_node(inner_or_id).unwrap();
        assert_eq!(inner_or_node.kind(), ExprKind::Or);

        let inner_children = inner_or_node.children();
        assert_eq!(inner_children.len(), 2);

        // 4. Inner children: (not B) and (C)
        assert_eq!(expr.try_node(inner_children[0]).unwrap().kind(), ExprKind::Not);
        assert_eq!(expr.try_node(inner_children[1]).unwrap().kind(), ExprKind::AtomicFormula);
    }

    /// Input: (A -> (and B C))
    /// Expected output: (or (and (B) (C)) (not (A)))
    #[test]
    fn test_imply_with_and_consequence() {
        let mut builder = ExprBuilder::new();

        // Setup: (imply (1) (and (2) (3)))
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);
        let and_bc = builder.and(vec![b, c]);
        let imply = builder.imply(a, and_bc);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        eliminate_imply(expr.root_id().unwrap(), &mut expr).unwrap();

        // --- Structure Validation ---
        let root_id = expr.root_id().unwrap();
        let root_node = expr.try_node(root_id).unwrap();

        // 1. Root should be OR
        assert_eq!(root_node.kind(), ExprKind::Or);
        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // 2. First child: (not A)
        let not_id = children[0];
        assert_eq!(expr.try_node(not_id).unwrap().kind(), ExprKind::Not);

        let not_children = expr.try_node(not_id).unwrap().children();
        assert_eq!(expr.try_node(not_children[0]).unwrap().kind(), ExprKind::AtomicFormula);

        // 3. Second child: (and B C)
        let and_id = children[1];
        let and_node = expr.try_node(and_id).unwrap();
        assert_eq!(and_node.kind(), ExprKind::And);

        let and_children = and_node.children();
        assert_eq!(and_children.len(), 2);
        assert_eq!(expr.try_node(and_children[0]).unwrap().kind(), ExprKind::AtomicFormula);
        assert_eq!(expr.try_node(and_children[1]).unwrap().kind(), ExprKind::AtomicFormula);
    }

    /// Input: (A -> (and))
    /// Expected output: (or (not (A)) (and))
    #[test]
    fn test_imply_with_empty_and_consequence() {
        let mut builder = ExprBuilder::new();

        // Setup: (imply (1) (and))
        let a = builder.atomic_formula(1, vec![]);
        let empty_and = builder.and(vec![]);
        let imply = builder.imply(a, empty_and);

        builder.set_root(imply).unwrap();
        let mut expr = builder.finish();

        eliminate_imply(expr.root_id().unwrap(), &mut expr).unwrap();

        // --- Structure Validation ---
        let root_id = expr.root_id().unwrap();
        let root_node = expr.try_node(root_id).unwrap();

        // 1. Root should be OR
        assert_eq!(root_node.kind(), ExprKind::Or);
        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // 2. First child: (not A)
        let not_id = children[0];
        assert_eq!(expr.try_node(not_id).unwrap().kind(), ExprKind::Not);

        // 3. Second child: empty AND
        let and_id = children[1];
        let and_node = expr.try_node(and_id).unwrap();
        assert_eq!(and_node.kind(), ExprKind::And);
        assert!(and_node.children().is_empty());
    }

    /// Input: ((forall ?X A) -> (exists ?Y B))
    /// Expected output: (or (not (forall (?X) (A))) (exists (?Y) (B)))
    #[test]
    fn test_imply_with_quantifiers() {
        let mut builder = ExprBuilder::new();

        // 1. Create typed variables using the new helpers
        // ?x has ID 10 and Type 100, ?y has ID 11 and Type 101
        let var_x = builder.typed_variable(10, &[100]);
        let var_y = builder.typed_variable(11, &[101]);

        // 2. Build atomic formulas: (A) and (B)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);

        // 3. Wrap them in quantifiers
        let forall_vars = builder.typed_variable_list(vec![var_x]);
        let forall_node = builder.forall(forall_vars, a);

        let exists_vars = builder.typed_variable_list(vec![var_y]);
        let exists_node = builder.exists(exists_vars, b);

        // 4. Create the implication: (forall (?x) (A ?x)) => (exists (?y) (B ?y))
        let imply = builder.imply(forall_node, exists_node);
        builder.set_root(imply).unwrap();

        let mut expr = builder.finish();

        // --- Transformation ---
        // (P => Q) becomes (not P or Q)
        eliminate_imply(expr.root_id().unwrap(), &mut expr).unwrap();

        // --- Validation ---
        let root_id = expr.root_id().unwrap();
        let root_node = expr.try_node(root_id).unwrap();

        // The root must now be an OR
        assert_eq!(root_node.kind(), ExprKind::Or);

        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // First child must be NOT: (not (forall (?x) (A)))
        let not_node_id = children[0];
        let not_node = expr.try_node(not_node_id).unwrap();
        assert_eq!(not_node.kind(), ExprKind::Not);

        let not_child_id = expr.try_node(not_node_id).unwrap().children()[0];
        let not_child_node = expr.try_node(not_child_id).unwrap();
        assert_eq!(not_child_node.kind(), ExprKind::Forall);

        // Second child must be the unchanged EXISTS: (exists (?y) (B))
        let exists_node_id = children[1];
        let exists_node = expr.try_node(exists_node_id).unwrap();
        assert_eq!(exists_node.kind(), ExprKind::Exists);
    }
}
