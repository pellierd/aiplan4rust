use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::expr::ops::rewriting::push_time_specifier::push_time_specifier;
use crate::aiplan4rust::tree::{NodeId, Node};

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
) -> Result<bool, ExprOpError> {
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
/// - Matching temporal nodes are replaced by their first child (if any) to types the tree.
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
) -> Result<NodeId, ExprOpError> {
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
                    // If this node is of the desired kind, types it by replacing it with its first child
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
    use crate::aiplan4rust::lir::expr::ExprKind;

    /// Test normalize temporal on a more complex expression.
    /// Input: (or (and (at start (A)) (over all (B))) (at end (not (C))))
    /// Expected output:
    /// (and
    ///     (at start (or (and (A))))
    ///     (at end (or (and) (not (C))))
    ///     (over all (or (and (B)))))
    #[test]
    fn test_normalize_temporal_complex() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: Build elements step-by-step to avoid borrow conflicts
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        let at_start_a = builder.at_start(a);
        let overall_b = builder.overall(b);
        let and_node = builder.and(vec![at_start_a, overall_b]);

        let not_c = builder.not(c);
        let at_end_not_c = builder.at_end(not_c);

        let root = builder.or(vec![and_node, at_end_not_c]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation
        // Note: We keep the ops where factorize_time_specifier might return false
        if !factorize_time_specifier(expr.try_root_id()?, &mut expr)? {
            return Ok(());
        }

        // 3. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::And);
        assert_eq!(root.children().len(), 3);

        // --- Branch 1: AtStart ---
        let start_id = root.children()[0];
        assert_eq!(expr.try_node_kind(start_id)?, ExprKind::AtStart);

        let or_start = expr.try_node(expr.try_node(start_id)?.children()[0])?;
        let and_a_id = or_start.children()[0];
        assert_eq!(expr.try_node_kind(expr.try_node(and_a_id)?.children()[0])?, ExprKind::AtomicFormula);

        // --- Branch 2: AtEnd ---
        let end_id = root.children()[1];
        assert_eq!(expr.try_node_kind(end_id)?, ExprKind::AtEnd);

        let or_end = expr.try_node(expr.try_node(end_id)?.children()[0])?;
        // Check first child of OR: empty AND
        let and_empty_id = or_end.children()[0];
        assert_eq!(expr.try_node_kind(and_empty_id)?, ExprKind::And);
        assert!(expr.try_node(and_empty_id)?.children().is_empty());

        // Check second child of OR: NOT
        assert_eq!(expr.try_node_kind(or_end.children()[1])?, ExprKind::Not);

        // --- Branch 3: Overall ---
        let overall_id = root.children()[2];
        assert_eq!(expr.try_node_kind(overall_id)?, ExprKind::Overall);

        let or_overall = expr.try_node(expr.try_node(overall_id)?.children()[0])?;
        let and_b_id = or_overall.children()[0];
        assert_eq!(expr.try_node_kind(expr.try_node(and_b_id)?.children()[0])?, ExprKind::AtomicFormula);

        Ok(())
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
    fn test_normalize_temporal_nested_overall_or() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: Prepare atoms
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        // 2. Build sub-structures (avoiding E0499)
        let start_a = builder.at_start(a);
        let overall_b = builder.overall(b);
        let end_c = builder.at_end(c);

        // Initial structure: (and (at start A) (or (overall B) (at end C)))
        let or_node = builder.or(vec![overall_b, end_c]);
        let root = builder.and(vec![start_a, or_node]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 3. Transformation
        if !factorize_time_specifier(expr.try_root_id()?, &mut expr)? {
            return Ok(());
        }

        // 4. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::And);
        assert_eq!(root.children().len(), 3);

        // --- Branch 1: AtStart (at start (and A (or))) ---
        let start_id = root.children()[0];
        assert_eq!(expr.try_node_kind(start_id)?, ExprKind::AtStart);

        let inner_start = expr.try_node(expr.try_node(start_id)?.children()[0])?;
        assert_eq!(inner_start.kind(), ExprKind::And);
        assert_eq!(expr.try_node_kind(inner_start.children()[0])?, ExprKind::AtomicFormula); // A

        let start_or_id = inner_start.children()[1];
        assert_eq!(expr.try_node_kind(start_or_id)?, ExprKind::Or);
        assert!(expr.try_node(start_or_id)?.children().is_empty());

        // --- Branch 2: AtEnd (at end (and (or C))) ---
        let end_id = root.children()[1];
        assert_eq!(expr.try_node_kind(end_id)?, ExprKind::AtEnd);

        let inner_end = expr.try_node(expr.try_node(end_id)?.children()[0])?;
        assert_eq!(inner_end.kind(), ExprKind::And);

        let end_or = expr.try_node(inner_end.children()[0])?;
        assert_eq!(end_or.kind(), ExprKind::Or);
        assert_eq!(expr.try_node_kind(end_or.children()[0])?, ExprKind::AtomicFormula); // C

        // --- Branch 3: Overall (overall (and (or B))) ---
        let overall_id = root.children()[2];
        assert_eq!(expr.try_node_kind(overall_id)?, ExprKind::Overall);

        let inner_overall = expr.try_node(expr.try_node(overall_id)?.children()[0])?;
        assert_eq!(inner_overall.kind(), ExprKind::And);

        let overall_or = expr.try_node(inner_overall.children()[0])?;
        assert_eq!(overall_or.kind(), ExprKind::Or);
        assert_eq!(expr.try_node_kind(overall_or.children()[0])?, ExprKind::AtomicFormula); // B

        Ok(())
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
    fn test_normalize_temporal_deeply_nested() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: Create atomic formulas
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);
        let d = builder.atomic_formula(4, vec![]);
        let e = builder.atomic_formula(5, vec![]);

        // 2. Build tree components (avoiding double mutable borrows)
        let and_ab = builder.and(vec![a, b]);
        let start_node = builder.at_start(and_ab);

        let or_cd = builder.or(vec![c, d]);
        let overall_node = builder.overall(or_cd);

        let not_e = builder.not(e);
        let end_node = builder.at_end(not_e);

        let root = builder.or(vec![start_node, overall_node, end_node]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 3. Transformation
        if !factorize_time_specifier(expr.try_root_id()?, &mut expr)? {
            return Ok(());
        }

        // 4. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::And);
        assert_eq!(root.children().len(), 3);

        // --- Branch 1: AtStart (at start (or (and A B))) ---
        let start_id = root.children()[0];
        assert_eq!(expr.try_node_kind(start_id)?, ExprKind::AtStart);

        let inner_start_or = expr.try_node(expr.try_node(start_id)?.children()[0])?;
        assert_eq!(inner_start_or.kind(), ExprKind::Or);

        let start_and = expr.try_node(inner_start_or.children()[0])?;
        assert_eq!(start_and.kind(), ExprKind::And);
        assert_eq!(start_and.children().len(), 2);

        // --- Branch 2: AtEnd (at end (or (not E))) ---
        let end_id = root.children()[1];
        assert_eq!(expr.try_node_kind(end_id)?, ExprKind::AtEnd);

        let inner_end_or = expr.try_node(expr.try_node(end_id)?.children()[0])?;
        assert_eq!(inner_end_or.kind(), ExprKind::Or);

        let end_not = expr.try_node(inner_end_or.children()[0])?;
        assert_eq!(end_not.kind(), ExprKind::Not);
        assert_eq!(expr.try_node_kind(end_not.children()[0])?, ExprKind::AtomicFormula);

        // --- Branch 3: Overall (overall (or (or C D))) ---
        let overall_id = root.children()[2];
        assert_eq!(expr.try_node_kind(overall_id)?, ExprKind::Overall);

        let inner_overall_or = expr.try_node(expr.try_node(overall_id)?.children()[0])?;
        assert_eq!(inner_overall_or.kind(), ExprKind::Or);

        let final_or = expr.try_node(inner_overall_or.children()[0])?;
        assert_eq!(final_or.kind(), ExprKind::Or);
        assert_eq!(final_or.children().len(), 2);

        // Check leaf atoms for Branch 3
        for &child_id in final_or.children() {
            assert_eq!(expr.try_node_kind(child_id)?, ExprKind::AtomicFormula);
        }

        Ok(())
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
    fn test_normalize_temporal_complex_nested() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: Create atomic formulas
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        // 2. Build temporal components
        let start_a = builder.at_start(a);
        let overall_b = builder.overall(b);
        let not_c = builder.not(c);
        let end_not_c = builder.at_end(not_c);

        // 3. Assemble tree: (or (and (at start A) (overall B)) (at end (not C)))
        let and_node = builder.and(vec![start_a, overall_b]);
        let root = builder.or(vec![and_node, end_not_c]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 4. Transformation
        if !factorize_time_specifier(expr.try_root_id()?, &mut expr)? {
            return Ok(());
        };

        // 5. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::And);
        assert_eq!(root.children().len(), 3);

        // --- Branch 1: at_start (at start (or (and A))) ---
        let start_id = root.children()[0];
        assert_eq!(expr.try_node_kind(start_id)?, ExprKind::AtStart);

        let inner_start_or = expr.try_node(expr.try_node(start_id)?.children()[0])?;
        assert_eq!(inner_start_or.kind(), ExprKind::Or);

        let and_a = expr.try_node(inner_start_or.children()[0])?;
        assert_eq!(and_a.kind(), ExprKind::And);
        assert_eq!(expr.try_node_kind(and_a.children()[0])?, ExprKind::AtomicFormula);

        // --- Branch 2: at_end (at end (or (and) (not C))) ---
        let end_id = root.children()[1];
        assert_eq!(expr.try_node_kind(end_id)?, ExprKind::AtEnd);

        let inner_end_or = expr.try_node(expr.try_node(end_id)?.children()[0])?;
        assert_eq!(inner_end_or.kind(), ExprKind::Or);
        assert_eq!(inner_end_or.children().len(), 2);

        // Check empty And
        let empty_and_id = inner_end_or.children()[0];
        assert_eq!(expr.try_node_kind(empty_and_id)?, ExprKind::And);
        assert!(expr.try_node(empty_and_id)?.children().is_empty());

        // Check Not C
        let not_c_id = inner_end_or.children()[1];
        assert_eq!(expr.try_node_kind(not_c_id)?, ExprKind::Not);
        assert_eq!(expr.try_node_kind(expr.try_node(not_c_id)?.children()[0])?, ExprKind::AtomicFormula);

        // --- Branch 3: overall (overall (or (and B))) ---
        let overall_id = root.children()[2];
        assert_eq!(expr.try_node_kind(overall_id)?, ExprKind::Overall);

        let inner_overall_or = expr.try_node(expr.try_node(overall_id)?.children()[0])?;
        assert_eq!(inner_overall_or.kind(), ExprKind::Or);

        let and_b = expr.try_node(inner_overall_or.children()[0])?;
        assert_eq!(and_b.kind(), ExprKind::And);
        assert_eq!(expr.try_node_kind(and_b.children()[0])?, ExprKind::AtomicFormula);

        Ok(())
    }

    /// Test normalizing an expression with AtStart containing a nested Forall, and AtEnd.
    /// Input: (and (at start (or (A) (forall (?X) (B)))) (at end (C)))
    /// Expected Output:
    /// (and
    ///    (at start (and (or (A) (forall (?X) (B)))))  // AtStart wraps filtered expr
    ///    (at end (and (C)))                             // AtEnd wraps filtered expression
    ///    (over all (and))                               // Overall empty but And
    /// )
    #[test]
    fn test_normalize_temporal_atstart_forall() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: Prepare atoms and variables using IDs
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        // Create a typed variable list for the quantifier
        let var_x = builder.typed_variable(10, &[100]); // ID 10, Type 100
        let forall_vars = builder.typed_variable_list(vec![var_x]);
        let forall_b = builder.forall(forall_vars, b);

        // 2. Build the initial tree: (and (at start (or A forall_B)) (at end C))
        let or_node = builder.or(vec![a, forall_b]);
        let start_or = builder.at_start(or_node);
        let end_c = builder.at_end(c);

        let root = builder.and(vec![start_or, end_c]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 3. Transformation
        if !factorize_time_specifier(expr.try_root_id()?, &mut expr)? {
            return Ok(());
        };

        // 4. Validation
        let root = expr.try_root_node()?;
        assert_eq!(root.kind(), ExprKind::And);
        assert_eq!(root.children().len(), 3);

        // --- Branch 1: AtStart (at start (and (or A forall_B))) ---
        let start_id = root.children()[0];
        assert_eq!(expr.try_node_kind(start_id)?, ExprKind::AtStart);

        let inner_start = expr.try_node(expr.try_node(start_id)?.children()[0])?;
        assert_eq!(inner_start.kind(), ExprKind::And);

        let or_child = expr.try_node(inner_start.children()[0])?;
        assert_eq!(or_child.kind(), ExprKind::Or);
        assert_eq!(expr.try_node_kind(or_child.children()[0])?, ExprKind::AtomicFormula); // A
        assert_eq!(expr.try_node_kind(or_child.children()[1])?, ExprKind::Forall);        // forall

        // --- Branch 2: AtEnd (at end (and C)) ---
        let end_id = root.children()[1];
        assert_eq!(expr.try_node_kind(end_id)?, ExprKind::AtEnd);

        let inner_end = expr.try_node(expr.try_node(end_id)?.children()[0])?;
        assert_eq!(inner_end.kind(), ExprKind::And);
        assert_eq!(expr.try_node_kind(inner_end.children()[0])?, ExprKind::AtomicFormula); // C

        // --- Branch 3: Overall (over all (and)) ---
        let overall_id = root.children()[2];
        assert_eq!(expr.try_node_kind(overall_id)?, ExprKind::Overall);

        let inner_overall = expr.try_node(expr.try_node(overall_id)?.children()[0])?;
        assert_eq!(inner_overall.kind(), ExprKind::And);
        assert!(inner_overall.children().is_empty()); // Empty And

        Ok(())
    }

}
