use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::tree::NodeId;

/// Removes all `Imply` nodes in the subtree rooted at `node_id` by transforming
/// each `A -> B` into `(Or(Not(A), B))`.
///
/// This function is the first step in the expr pipeline. It ensures that
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
/// - It should be the **first step** in the expr pipeline, before `push_negation` and
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
///   of the expr workflow orchestrated by the `normalize` module.
///
/// # Example
/// ```ignore
/// // Suppose `expr` contains multiple Imply nodes in a subtree rooted at node_id
/// eliminate_imply(node_id, &mut expr)?;
/// // All Imply nodes in that subtree are now replaced by Or(Not(premise), consequence)
/// ```
pub fn eliminate_imply(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprOpError> {
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
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;

    /// Input: (A -> B)
    /// Expected output: (or (B) (not (A)))
    #[test]
    fn test_nested_imply() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Compact Setup: (imply (1) (imply (2) (3)))
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);
        let imply_bc = builder.imply(b, c);
        let root_imply = builder.imply(a, imply_bc);

        builder.set_root(root_imply)?;
        let mut expr = builder.finish();

        // 2. Transformation: (A => (B => C)) -> (not A or (not B or C))
        eliminate_imply(expr.try_root_id()?, &mut expr)?;

        // 3. Chained Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::Or);
        assert_eq!(root.children().len(), 2);

        // Direct check of first child: (not A)
        // Using root.children()[0] or your try_child helper
        assert_eq!(expr.try_node_kind(root.children()[0])?, ExprKind::Not);

        // Navigate to second child: (or (not B) C)
        let inner_or = expr.try_node(root.children()[1])?;
        assert_eq!(inner_or.kind(), ExprKind::Or);
        assert_eq!(inner_or.children().len(), 2);

        // Verify grand-children: (not B) and (C)
        assert_eq!(expr.try_node_kind(inner_or.children()[0])?, ExprKind::Not);
        assert_eq!(expr.try_node_kind(inner_or.children()[1])?, ExprKind::AtomicFormula);

        Ok(())
    }

    /// Input: (A -> (and B C))
    /// Expected output: (or (and (B) (C)) (not (A)))
    #[test]
    fn test_imply_with_and_consequence() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Compact Setup: (imply (1) (and (2) (3)))
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);
        let and_bc = builder.and(vec![b, c]);
        let root_imply = builder.imply(a, and_bc);

        builder.set_root(root_imply)?;
        let mut expr = builder.finish();

        // 2. Transformation: (A => (B and C)) -> (not A or (B and C))
        eliminate_imply(expr.try_root_id()?, &mut expr)?;

        // 3. Chained Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::Or);
        assert_eq!(root.children().len(), 2);

        // Verify first child: (not A)
        let not_id = root.children()[0];
        assert_eq!(expr.try_node_kind(not_id)?, ExprKind::Not);
        assert_eq!(expr.try_node_kind(expr.try_node(not_id)?.children()[0])?, ExprKind::AtomicFormula);

        // Verify second child: (and B C)
        let and_id = root.children()[1];
        let and_node = expr.try_node(and_id)?;
        assert_eq!(and_node.kind(), ExprKind::And);
        assert_eq!(and_node.children().len(), 2);

        // Verify leaf atoms in the AND block
        assert_eq!(expr.try_node_kind(and_node.children()[0])?, ExprKind::AtomicFormula);
        assert_eq!(expr.try_node_kind(and_node.children()[1])?, ExprKind::AtomicFormula);

        Ok(())
    }

    /// Input: (A -> (and))
    /// Expected output: (or (not (A)) (and))
    #[test]
    fn test_imply_with_empty_and_consequence() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Compact Setup: (imply (1) (and))
        let a = builder.atomic_formula(1, vec![]);
        let and = builder.and(vec![]);
        let root_imply = builder.imply(a, and);

        builder.set_root(root_imply)?;
        let mut expr = builder.finish();

        // 2. Transformation: (A => true) -> (not A or true)
        eliminate_imply(expr.try_root_id()?, &mut expr)?;

        // 3. Chained Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::Or);
        assert_eq!(root.children().len(), 2);

        // Verify first child: (not A)
        assert_eq!(expr.try_node_kind(root.children()[0])?, ExprKind::Not);

        // Verify second child: empty AND
        let and_node = expr.try_node(root.children()[1])?;
        assert_eq!(and_node.kind(), ExprKind::And);
        assert!(and_node.children().is_empty());

        Ok(())
    }

    /// Input: ((forall ?X A) -> (exists ?Y B))
    /// Expected output: (or (not (forall (?X) (A))) (exists (?Y) (B)))
    #[test]
    fn test_imply_with_quantifiers() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: Create typed variables (?x:100, ?y:101)
        let var_x = builder.typed_variable(10, &[100]);
        let var_y = builder.typed_variable(11, &[101]);

        // 2. Build Quantifiers: (forall (?x) (A)) and (exists (?y) (B))
        let forall_var = builder.typed_variable_list(vec![var_x]);
        let a  = builder.atomic_formula(1, vec![]);
        let forall_node = builder.forall(forall_var, a);
        let b = builder.atomic_formula(2, vec![]);
        let exists_var = builder.typed_variable_list(vec![var_y]);
        let exists_node = builder.exists(exists_var, b);

        // 3. Create the implication: (forall... => exists...)
        let root_imply = builder.imply(forall_node, exists_node);
        builder.set_root(root_imply)?;

        let mut expr = builder.finish();

        // 4. Transformation: (P => Q) -> (not P or Q)
        eliminate_imply(expr.try_root_id()?, &mut expr)?;

        // 5. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::Or);
        assert_eq!(root.children().len(), 2);

        // Verify first child: (not (forall ...))
        let not_id = root.children()[0];
        assert_eq!(expr.try_node_kind(not_id)?, ExprKind::Not);

        let inner_forall_id = expr.try_node(not_id)?.children()[0];
        assert_eq!(expr.try_node_kind(inner_forall_id)?, ExprKind::Forall);

        // Verify second child: unchanged (exists ...)
        let exists_id = root.children()[1];
        assert_eq!(expr.try_node_kind(exists_id)?, ExprKind::Exists);

        Ok(())
    }
}
