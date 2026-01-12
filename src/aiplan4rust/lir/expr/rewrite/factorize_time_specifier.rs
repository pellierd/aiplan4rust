use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::rewrite::push_time_specifier::push_time_specifier;
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxNode};

/// Factorizes temporal specifiers in an expression tree.
///
/// This function assumes that the expression has already been preprocessed:
/// 1. **Implications eliminated** via `eliminate_imply`.
/// 2. **Negations pushed down** via `push_negation`.
///
/// It then pushes temporal specifiers down to atomic formulas (internal `push_time_specifier`),
/// splits the expression into three separate temporal contexts (`AtStart`, `AtEnd`, `Overall`),
/// and combines them under a new `And` root.
///
/// The resulting expression has the following structure:
/// ```text
///           And
///         /  |   \
///   AtStart AtEnd Overall
///     ...     ...    ...
/// ```
/// Each temporal specifier subtree contains only literals relevant to its temporal context.
///
/// # Dependencies
///
/// - `eliminate_imply` must have been applied first.
/// - `push_negation` must have been applied after `eliminate_imply`.
/// - `push_time_specifier` (internal) is called within this function to propagate temporal specifiers.
///
/// Note: **the module `normalizer` is responsible for orchestrating these dependencies**.
/// Users of the API do not need to call `eliminate_imply` or `push_negation` directly;
/// they can simply call `normalize_temporal_expr` (or similar orchestrator) which will
/// execute the steps in the correct order.
///
/// # Arguments
///
/// * `root_id` - The ID of the root node of the expression to factorize.
/// * `expr` - A mutable reference to the expression tree to modify.
///
/// # Returns
///
/// Returns `Ok(true)` if factorization was performed, `Ok(false)` if no temporal specifiers
/// were present in the subtree. Returns an `ExprError` if any operation fails.
pub fn factorize_time_specifier(
    root_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprError> {
    // 1. Push temporal specifiers down to atomic formulas
    // This uses the internal `push_time_specifier`, which depends on negations already being pushed.
    // If there are no temporal specifiers, exit early.
    if !push_time_specifier(root_id, expr)? {
        return Ok(false);
    }

    // 2. Clone and filter the expression subtree for each temporal kind
    // Each call produces a new subtree containing only the specified temporal specifier
    let at_start = filter_temporal(expr.clone_subtree(root_id)?, ExprKind::AtStart, expr)?;
    let at_end = filter_temporal(expr.clone_subtree(root_id)?, ExprKind::AtEnd, expr)?;
    let overall = filter_temporal(expr.clone_subtree(root_id)?, ExprKind::Overall, expr)?;

    // 3. Create a new 'And' node to combine the three temporal subtrees
    let and_node = ExprNode::new(ExprKind::And, ExprContent::None, None);
    let new_root_id = expr.alloc_root_with_children(
        and_node,
        vec![at_start, at_end, overall], // children are the three filtered temporal subtrees
    );

    // 4. Update the tree's root to the new 'And' node
    expr.set_root_id(new_root_id)?;

    // Factorization completed successfully
    Ok(true)
}


/// Filters a temporal expression subtree to retain only nodes of a specified temporal kind.
///
/// This function performs a depth-first traversal of a cloned expression subtree,
/// removing any temporal specifier nodes that do not match `keep_kind`. Nodes that
/// match `keep_kind` are replaced by their first child to avoid nested temporal specifiers.
///
/// # Behavior
/// - Temporal nodes (`ExprKind::AtStart`, `ExprKind::AtEnd`, `ExprKind::Overall`)
///   not matching `keep_kind` are removed from the tree.
/// - Matching temporal nodes are replaced by their first child (if any) to flatten the tree.
/// - Non-temporal nodes are left untouched.
/// - The resulting filtered subtree is wrapped in a new temporal specifier node of `keep_kind`.
///
/// # Arguments
///
/// * `root_id` - The ID of the root node of the subtree to filter.
/// * `keep_kind` - The `ExprKind` of the temporal specifier to retain.
/// * `expr` - The mutable reference to the expression tree being modified.
///
/// # Returns
///
/// Returns the `NodeId` of the new root node wrapped in the desired temporal specifier.
///
/// # Errors
///
/// Returns `ExprError` if any node operations (lookup, mutation, move) fail.
#[allow(dead_code)]
fn filter_temporal(
    root_id: NodeId,
    keep_kind: ExprKind,
    expr: &mut Expr,
) -> Result<NodeId, ExprError> {
    // Initialize a stack with the root node for DFS traversal
    let mut stack = vec![root_id];

    // Perform DFS in post-order
    while let Some(node_id) = stack.pop() {
        // Temporarily borrow node data to avoid holding multiple mutable borrows
        let (kind, children, parent_id) = {
            let node = expr.try_node(node_id)?; // Get immutable reference to node
            (node.kind(), node.children().to_vec(), node.parent()) // Extract kind, children, parent
        };

        // Push all children onto the stack for traversal
        for &child_id in &children {
            stack.push(child_id);
        }

        // Process only temporal nodes
        match kind {
            ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                if kind == keep_kind {
                    // If this node is of the desired kind, flatten it by replacing it with its first child
                    if !children.is_empty() {
                        let child_id = children[0];
                        expr.move_to(child_id, node_id)?; // Move child under this node's parent
                    }
                } else {
                    // If this node is not of the desired kind, detach it from the tree
                    if let Some(pid) = parent_id {
                        let parent = expr.try_node_mut(pid)?; // Mutable reference to parent
                        parent.children_mut().retain(|&cid| cid != node_id); // Remove this node from parent's children
                    }
                    expr.try_node_mut(node_id)?.set_parent(None); // Clear parent reference
                }
            }
            _ => {} // Non-temporal nodes are ignored
        }
    }

    // After filtering, wrap the subtree in a new temporal specifier of the desired kind
    let time_spec_node = ExprNode::new(keep_kind, ExprContent::None, None);
    let time_spec_id = expr.alloc(time_spec_node); // Allocate a new node
    expr.try_node_mut(time_spec_id)?.add_child(root_id); // Set original root as child
    expr.try_node_mut(root_id)?.set_parent(Some(time_spec_id)); // Set new node as parent

    // Return the ID of the new root node
    Ok(time_spec_id)
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::arena::ArenaNode;
    use super::*;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::syntax::SyntaxInternerDisplay;

    /// Test normalize temporal on a more complex expression.
    /// Input: (or (and (at start (A)) (over all (B))) (at end (not (C))))
    /// Expected output:
    /// (and
    ///     (at start (or (and (A))))
    ///     (at end (or (and) (not (C))))
    ///     (over all (or (and (B)))))
    #[test]
    fn test_normalize_temporal_complex() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);

        let at_start_a = builder.at_start(a);
        let overall_b = builder.overall(b);
        let and_node = builder.and(vec![at_start_a, overall_b]);

        let not_c = builder.not(c);
        let at_end_not_c = builder.at_end(not_c);
        let root = builder.or(vec![and_node, at_end_not_c]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        if !factorize_time_specifier(expr.try_root_id().unwrap(), &mut expr).unwrap() {
            return ;
        };
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.try_root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 3);

        // at_start
        let at_start_id = root_node.children()[0];
        let at_start_node = expr.try_node(at_start_id).unwrap();
        assert_eq!(at_start_node.kind(), ExprKind::AtStart);
        let child_id = at_start_node.children()[0];
        let child_node = expr.try_node(child_id).unwrap();
        assert_eq!(child_node.kind(), ExprKind::Or);
        assert_eq!(child_node.children().len(), 1);
        let and_child_id = child_node.children()[0];
        let and_child = expr.try_node(and_child_id).unwrap();
        assert_eq!(and_child.kind(), ExprKind::And);
        let a_node = expr.try_node(and_child.children()[0]).unwrap();
        assert_eq!(a_node.kind(), ExprKind::AtomicFormula);

        // at_end
        let at_end_id = root_node.children()[1];
        let at_end_node = expr.try_node(at_end_id).unwrap();
        assert_eq!(at_end_node.kind(), ExprKind::AtEnd);
        let child_id = at_end_node.children()[0];
        let child_node = expr.try_node(child_id).unwrap();
        assert_eq!(child_node.kind(), ExprKind::Or);
        assert_eq!(child_node.children().len(), 2);
        let and_empty = expr.try_node(child_node.children()[0]).unwrap();
        assert_eq!(and_empty.kind(), ExprKind::And);
        assert_eq!(and_empty.children().len(), 0);
        let not_c_node = expr.try_node(child_node.children()[1]).unwrap();
        assert_eq!(not_c_node.kind(), ExprKind::Not);
        let c_node = expr.try_node(not_c_node.children()[0]).unwrap();
        assert_eq!(c_node.kind(), ExprKind::AtomicFormula);

        // overall
        let overall_id = root_node.children()[2];
        let overall_node = expr.try_node(overall_id).unwrap();
        assert_eq!(overall_node.kind(), ExprKind::Overall);
        let child_id = overall_node.children()[0];
        let child_node = expr.try_node(child_id).unwrap();
        assert_eq!(child_node.kind(), ExprKind::Or);
        let and_b = expr.try_node(child_node.children()[0]).unwrap();
        assert_eq!(and_b.kind(), ExprKind::And);
        let b_node = expr.try_node(and_b.children()[0]).unwrap();
        assert_eq!(b_node.kind(), ExprKind::AtomicFormula);

        assert_eq!(
            output,
            "(and (at start (or (and (A)))) (at end (or (and) (not (C)))) (over all (or (and (B)))))"
        );
    }

    /// Test normalizing a nested expression with multiple time specifiers, preserving logical structure.
    /// Input: (and (at start (A)) (or (overall (B)) (at end (C))))
    /// Expected Output:
    /// (and
    ///    (at start (and (A) (or)))      // start wrapped in And with empty Or
    ///    (at end (and (or (C))))        // end wrapped in And with Or(C)
    ///    (over all (and (or (B))))      // overall wrapped in And with Or(B)
    /// )
    #[test]
    fn test_normalize_temporal_nested_overall_or() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);

        let start_a = builder.at_start(a);
        let overall_b = builder.overall(b);
        let end_c = builder.at_end(c);

        let or_node = builder.or(vec![overall_b, end_c]);
        let root = builder.and(vec![start_a, or_node]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        if !factorize_time_specifier(expr.try_root_id().unwrap(), &mut expr).unwrap() {
            return ;
        };

        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.try_root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 3);

        // at_start
        let at_start_node = expr.try_node(root_node.children()[0]).unwrap();
        assert_eq!(at_start_node.kind(), ExprKind::AtStart);
        assert_eq!(at_start_node.children().len(), 1);
        let inner_start = expr.try_node(at_start_node.children()[0]).unwrap();
        assert_eq!(inner_start.kind(), ExprKind::And);
        assert_eq!(inner_start.children().len(), 2);

        let child_start_a = expr.try_node(inner_start.children()[0]).unwrap();
        assert_eq!(child_start_a.kind(), ExprKind::AtomicFormula);
        let child_start_or = expr.try_node(inner_start.children()[1]).unwrap();
        assert_eq!(child_start_or.kind(), ExprKind::Or);
        assert_eq!(child_start_or.children().len(), 0); // empty Or

        // at_end
        let at_end_node = expr.try_node(root_node.children()[1]).unwrap();
        assert_eq!(at_end_node.kind(), ExprKind::AtEnd);
        assert_eq!(at_end_node.children().len(), 1);
        let inner_end = expr.try_node(at_end_node.children()[0]).unwrap();
        assert_eq!(inner_end.kind(), ExprKind::And);
        assert_eq!(inner_end.children().len(), 1);
        let child_end_or = expr.try_node(inner_end.children()[0]).unwrap();
        assert_eq!(child_end_or.kind(), ExprKind::Or);
        assert_eq!(child_end_or.children().len(), 1);
        let atom_c = expr.try_node(child_end_or.children()[0]).unwrap();
        assert_eq!(atom_c.kind(), ExprKind::AtomicFormula);

        // overall
        let overall_node = expr.try_node(root_node.children()[2]).unwrap();
        assert_eq!(overall_node.kind(), ExprKind::Overall);
        assert_eq!(overall_node.children().len(), 1);
        let inner_overall = expr.try_node(overall_node.children()[0]).unwrap();
        assert_eq!(inner_overall.kind(), ExprKind::And);
        assert_eq!(inner_overall.children().len(), 1);
        let child_overall_or = expr.try_node(inner_overall.children()[0]).unwrap();
        assert_eq!(child_overall_or.kind(), ExprKind::Or);
        assert_eq!(child_overall_or.children().len(), 1);
        let atom_b = expr.try_node(child_overall_or.children()[0]).unwrap();
        assert_eq!(atom_b.kind(), ExprKind::AtomicFormula);

        assert_eq!(
            output,
            "(and (at start (and (A) (or))) (at end (and (or (C)))) (over all (and (or (B)))))"
        );
    }

    /// Test normalizing a deeply nested expression with multiple time specifiers.
    /// Input: (or (at start (and (A) (B))) (over all (or (C) (D))) (at end (not (E))))
    /// Expected Output:
    /// (and
    ///    (at start (or (and (A) (B))))      // start preserved, wrapped in Or
    ///    (at end (or (not (E))))             // end preserved, wrapped in Or
    ///    (over all (or (or (C) (D))))        // overall preserved, original Or preserved inside new Or
    /// )
    #[test]
    fn test_normalize_temporal_deeply_nested() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let d = builder.atomic_formula("D", vec![]);
        let e = builder.atomic_formula("E", vec![]);

        let and_a_b = builder.and(vec![a, b]);
        let start_and = builder.at_start(and_a_b);
        let or_c_d = builder.or(vec![c, d]);
        let overall_or = builder.overall(or_c_d);
        let not_e = builder.not(e);
        let end_not = builder.at_end(not_e);

        let root = builder.or(vec![start_and, overall_or, end_not]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        if !factorize_time_specifier(expr.try_root_id().unwrap(), &mut expr).unwrap() {
            return ;
        };

        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.try_root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 3);

        // at_start
        let at_start_node = expr.try_node(root_node.children()[0]).unwrap();
        assert_eq!(at_start_node.kind(), ExprKind::AtStart);
        assert_eq!(at_start_node.children().len(), 1);
        let inner_start = expr.try_node(at_start_node.children()[0]).unwrap();
        assert_eq!(inner_start.kind(), ExprKind::Or);
        assert_eq!(inner_start.children().len(), 1);
        let child_start = expr.try_node(inner_start.children()[0]).unwrap();
        assert_eq!(child_start.kind(), ExprKind::And);
        assert_eq!(child_start.children().len(), 2);

        // at_end
        let at_end_node = expr.try_node(root_node.children()[1]).unwrap();
        assert_eq!(at_end_node.kind(), ExprKind::AtEnd);
        assert_eq!(at_end_node.children().len(), 1);
        let inner_end = expr.try_node(at_end_node.children()[0]).unwrap();
        assert_eq!(inner_end.kind(), ExprKind::Or);
        assert_eq!(inner_end.children().len(), 1);
        let child_end = expr.try_node(inner_end.children()[0]).unwrap();
        assert_eq!(child_end.kind(), ExprKind::Not);
        let atom_e = expr.try_node(child_end.children()[0]).unwrap();
        assert_eq!(atom_e.kind(), ExprKind::AtomicFormula);

        // overall
        let overall_node = expr.try_node(root_node.children()[2]).unwrap();
        assert_eq!(overall_node.kind(), ExprKind::Overall);
        assert_eq!(overall_node.children().len(), 1);
        let inner_overall = expr.try_node(overall_node.children()[0]).unwrap();
        assert_eq!(inner_overall.kind(), ExprKind::Or);
        assert_eq!(inner_overall.children().len(), 1);
        let child_overall = expr.try_node(inner_overall.children()[0]).unwrap();
        assert_eq!(child_overall.kind(), ExprKind::Or);
        assert_eq!(child_overall.children().len(), 2);

        for &child_id in child_overall.children().iter() {
            let child = expr.try_node(child_id).unwrap();
            assert_eq!(child.kind(), ExprKind::AtomicFormula);
        }

        assert_eq!(
            output,
            "(and (at start (or (and (A) (B)))) (at end (or (not (E)))) (over all (or (or (C) (D)))))"
        );
    }

    /// Test normalizing a complex nested expression with multiple time specifiers.
    /// Input: (or (and (at start (A)) (over all (B))) (at end (not (C))))
    /// Expected Output:
    /// (and
    ///    (at start (or (and (A))))       // start wrapped in Or
    ///    (at end (or (and) (not (C))))   // end wrapped in Or, contains empty And + Not(C)
    ///    (over all (or (and (B))))       // overall wrapped in Or
    /// )
    #[test]
    fn test_normalize_temporal_complex_nested() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);

        let start_a = builder.at_start(a);
        let overall_b = builder.overall(b);
        let not_c = builder.not(c);
        let end_not_c = builder.at_end(not_c);

        let and_node = builder.and(vec![start_a, overall_b]);
        let root = builder.or(vec![and_node, end_not_c]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        if !factorize_time_specifier(expr.try_root_id().unwrap(), &mut expr).unwrap() {
            return ;
        };
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.try_root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 3);

        // at_start
        let at_start_node = expr.try_node(root_node.children()[0]).unwrap();
        assert_eq!(at_start_node.kind(), ExprKind::AtStart);
        assert_eq!(at_start_node.children().len(), 1);
        let inner_start = expr.try_node(at_start_node.children()[0]).unwrap();
        assert_eq!(inner_start.kind(), ExprKind::Or);
        assert_eq!(inner_start.children().len(), 1);
        let and_a = expr.try_node(inner_start.children()[0]).unwrap();
        assert_eq!(and_a.kind(), ExprKind::And);
        assert_eq!(and_a.children().len(), 1);
        let atom_a = expr.try_node(and_a.children()[0]).unwrap();
        assert_eq!(atom_a.kind(), ExprKind::AtomicFormula);

        // at_end
        let at_end_node = expr.try_node(root_node.children()[1]).unwrap();
        assert_eq!(at_end_node.kind(), ExprKind::AtEnd);
        assert_eq!(at_end_node.children().len(), 1);
        let inner_end = expr.try_node(at_end_node.children()[0]).unwrap();
        assert_eq!(inner_end.kind(), ExprKind::Or);
        assert_eq!(inner_end.children().len(), 2);

        let first_child = expr.try_node(inner_end.children()[0]).unwrap();
        assert_eq!(first_child.kind(), ExprKind::And);
        assert_eq!(first_child.children().len(), 0); // empty And

        let second_child = expr.try_node(inner_end.children()[1]).unwrap();
        assert_eq!(second_child.kind(), ExprKind::Not);
        assert_eq!(second_child.children().len(), 1);
        let atom_c = expr.try_node(second_child.children()[0]).unwrap();
        assert_eq!(atom_c.kind(), ExprKind::AtomicFormula);

        // overall
        let overall_node = expr.try_node(root_node.children()[2]).unwrap();
        assert_eq!(overall_node.kind(), ExprKind::Overall);
        assert_eq!(overall_node.children().len(), 1);
        let inner_overall = expr.try_node(overall_node.children()[0]).unwrap();
        assert_eq!(inner_overall.kind(), ExprKind::Or);
        assert_eq!(inner_overall.children().len(), 1);
        let and_b = expr.try_node(inner_overall.children()[0]).unwrap();
        assert_eq!(and_b.kind(), ExprKind::And);
        assert_eq!(and_b.children().len(), 1);
        let atom_b = expr.try_node(and_b.children()[0]).unwrap();
        assert_eq!(atom_b.kind(), ExprKind::AtomicFormula);

        assert_eq!(
            output,
            "(and (at start (or (and (A)))) (at end (or (and) (not (C)))) (over all (or (and (B)))))"
        );
    }

    /// Test normalizing an expression with AtStart containing a nested Forall, and AtEnd.
    /// Input: (and (at start (or (A) (forall (?X) (B)))) (at end (C)))
    /// Expected Output:
    /// (and
    ///    (at start (and (or (A) (forall (?X) (B)))))  // AtStart wraps filtered expressions
    ///    (at end (and (C)))                             // AtEnd wraps filtered expression
    ///    (over all (and))                               // Overall empty but And
    /// )
    #[test]
    fn test_normalize_temporal_atstart_forall() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);

        let forall_b = builder.forall_with_string_vars(vec![("?X", "T")], b);

        let or_node = builder.or(vec![a, forall_b]);

        let start_or = builder.at_start(or_node);

        let end_c = builder.at_end(c);

        let root = builder.and(vec![start_or, end_c]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string_with_interner(&interner);
        if !factorize_time_specifier(expr.try_root_id().unwrap(), &mut expr).unwrap() {
            return ;
        };
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.try_root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 3);

        // at_start
        let at_start_node = expr.try_node(root_node.children()[0]).unwrap();
        assert_eq!(at_start_node.kind(), ExprKind::AtStart);
        assert_eq!(at_start_node.children().len(), 1);
        let inner_start = expr.try_node(at_start_node.children()[0]).unwrap();
        assert_eq!(inner_start.kind(), ExprKind::And);
        let and_child = expr.try_node(inner_start.children()[0]).unwrap();
        assert_eq!(and_child.kind(), ExprKind::Or);
        assert_eq!(and_child.children().len(), 2);
        let child1 = expr.try_node(and_child.children()[0]).unwrap();
        assert_eq!(child1.kind(), ExprKind::AtomicFormula);
        let child2 = expr.try_node(and_child.children()[1]).unwrap();
        assert_eq!(child2.kind(), ExprKind::Forall);
        assert_eq!(child2.children().len(), 1);
        let body = expr.try_node(child2.children()[0]).unwrap();
        assert_eq!(body.kind(), ExprKind::AtomicFormula);

        // at_end
        let at_end_node = expr.try_node(root_node.children()[1]).unwrap();
        assert_eq!(at_end_node.kind(), ExprKind::AtEnd);
        assert_eq!(at_end_node.children().len(), 1);
        let inner_end = expr.try_node(at_end_node.children()[0]).unwrap();
        assert_eq!(inner_end.kind(), ExprKind::And);
        assert_eq!(inner_end.children().len(), 1);
        let child_c = expr.try_node(inner_end.children()[0]).unwrap();
        assert_eq!(child_c.kind(), ExprKind::AtomicFormula);

        // overall
        let overall_node = expr.try_node(root_node.children()[2]).unwrap();
        assert_eq!(overall_node.kind(), ExprKind::Overall);
        assert_eq!(overall_node.children().len(), 1);
        let inner_overall = expr.try_node(overall_node.children()[0]).unwrap();
        assert_eq!(inner_overall.kind(), ExprKind::And);
        assert_eq!(inner_overall.children().len(), 0);

        assert_eq!(
            output,
            "(and (at start (and (or (A) (forall (?X) (B))))) (at end (and (C))) (over all (and)))"
        );
    }

}
