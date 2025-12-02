use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::syntax::tree::NodeId;

/// Removes all `Imply` nodes in the subtree rooted at `node_id` by transforming
/// each `A -> B` into `(or (not A) B)`.
///
/// # Behavior
/// - Traverses the subtree in **post-order** (DFS) to ensure children are processed before their parent.
/// - Each `Imply` node encountered is replaced by an `Or` node:
///   - The premise `A` becomes a `Not(A)` node.
///   - The consequence `B` is kept as-is.
///   - The current `Imply` node is converted into `Or(Not(A), B)`.
/// - Non-`Imply` nodes are left unchanged.
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
///
/// # Example
/// ```ignore
/// // Suppose `expr` contains multiple Imply nodes in a subtree rooted at node_id
/// remove_imply(node_id, &mut expr)?;
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
        eliminate_imply(expr.root_id().unwrap(), &mut expr).unwrap();
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
        eliminate_imply(expr.root_id().unwrap(), &mut expr).unwrap();
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
        eliminate_imply(expr.root_id().unwrap(), &mut expr).unwrap();
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
        eliminate_imply(expr.root_id().unwrap(), &mut expr).unwrap();
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
