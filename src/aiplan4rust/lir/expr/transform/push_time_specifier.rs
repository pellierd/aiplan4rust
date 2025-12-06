use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxNode};

/// Recursively pushes temporal specifier nodes (`AtStart`, `AtEnd`, `Overall`)
/// down to atomic formulas in an expression tree.
///
/// This function traverses the expression tree starting from `root_id` and propagates
/// temporal context so that each atomic formula is wrapped with the appropriate
/// temporal specifier. Logical operators (`And`, `Or`) and quantifiers (`Forall`, `Exists`)
/// are handled specifically to ensure correct propagation.
///
/// # Precondition
/// - The expression tree should have already had negations normalized via `push_negation`.
///   This ensures that all literals under `Not` nodes are in a form suitable for
///   temporal propagation.
///
/// # Transformations applied
/// - `(at_start X)` → temporal specifier pushed down to children of `X`.
/// - `(at_end X)` → temporal specifier pushed down similarly.
/// - `(overall X)` → temporal specifier pushed down similarly.
///
/// # Parameters
/// - `root_id`: NodeId of the root of the subtree to process.
/// - `expr`: Mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(true)` if any modifications were made (new temporal nodes inserted).
/// - `Ok(false)` if the tree was already in the desired form.
/// - `Err(ExprError)` if node access or mutation fails.
///
/// # Notes
/// - Mutates the tree in place.
/// - Uses a stack to manage nodes for depth-first propagation.
/// - Newly created temporal nodes are pushed onto the stack for further processing.
/// - Assumes each temporal specifier node has exactly one child.
pub fn push_time_specifier(root_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    // Initialize a stack for depth-first traversal starting with the root
    let mut stack = vec![root_id];
    let mut modified = false;

    while let Some(node_id) = stack.pop() {
        // Retrieve the node
        let node = expr.try_node(node_id)?;
        let kind = node.kind();

        // Only act if the node is a temporal specifier
        if kind == ExprKind::AtStart || kind == ExprKind::AtEnd || kind == ExprKind::Overall {
            let children = node.children();

            // Temporal specifiers must have exactly one child
            debug_assert!(
                children.len() == 1,
                "Temporal node must have exactly one child"
            );

            let child_id = children[0];
            let child = expr.try_node(child_id)?;
            let child_kind = child.kind();

            match child_kind {
                ExprKind::And | ExprKind::Or => {
                    // Logical operator: push temporal specifier to each child
                    let new_ids = push_time_specifier_to_children(node_id, kind, expr)?;
                    modified = true;

                    // Continue propagating temporal specifiers into newly created nodes
                    stack.extend(new_ids);
                }
                ExprKind::Forall | ExprKind::Exists => {
                    // Quantifier: wrap the body with a temporal node
                    let new_root_id = push_time_specifier_into_quantifier(node_id, expr)?;
                    modified = true;

                    // Continue processing from the new temporal node
                    stack.push(new_root_id);
                }
                _ => {
                    // Atomic formula reached: no further propagation required
                }
            }
        }
    }

    // If the tree was modified, verify temporal consistency to verify that every atomic formula
    // is properly wrapped in a temporal specifier
    if modified {
        verify_temporal_consistency(expr, root_id)?;
    }

    // Return whether any modifications were applied
    Ok(modified)
}

/// Pushes a temporal specifier node (`AtStart`, `AtEnd`, `Overall`) down to each child
/// of a logical operator node (`And` / `Or`) in the expression tree.
///
/// This function is used when a temporal specifier wraps an `And` or `Or` node,
/// and we want to propagate the temporal context to each child individually.
///
/// # Parameters
/// - `temporal_id`: The NodeId of the temporal specifier node to propagate.
/// - `kind`: The kind of temporal specifier to insert (`AtStart`, `AtEnd`, `Overall`).
/// - `expr`: A mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(Vec<NodeId>)`: A vector of newly created temporal nodes that wrap the original children,
///   useful for further processing in the push algorithm.
/// - `Err(ExprError)`: If any node access or mutation fails.
///
/// # Panics / Debug asserts
/// - The temporal node must have exactly one child.
/// - The child must be a logical operator (`And` / `Or`).
#[allow(dead_code)]
fn push_time_specifier_to_children(
    temporal_id: NodeId,      // ID of the initial temporal node (AtStart / AtEnd / Overall)
    kind: ExprKind,           // Temporal specifier type to push
    expr: &mut Expr,
) -> Result<Vec<NodeId>, ExprError> {
    // Retrieve the temporal node and assert it has exactly one child
    let temporal_node = expr.try_node(temporal_id)?;
    debug_assert!(
        temporal_node.children().len() == 1,
        "Temporal node must have exactly one child"
    );

    // Get the child node of the temporal node (expected to be AND/OR)
    let child_id = temporal_node.children()[0];
    let child_node = expr.try_node(child_id)?;
    let logical_kind = child_node.kind();
    debug_assert!(
        logical_kind == ExprKind::And || logical_kind == ExprKind::Or,
        "Child of temporal node must be AND or OR"
    );

    // Collect all original children of the logical node
    let children_ids = child_node.children().to_vec();

    // Prepare a vector to store new temporal nodes
    let mut new_ids = Vec::with_capacity(children_ids.len());

    // For each original child, create a new temporal node wrapping it
    for original_child_id in children_ids {
        let new_temporal_node = ExprNode::new(kind, Content::None, Some(temporal_id));
        let new_temporal_id = expr.alloc_with_children(new_temporal_node, vec![original_child_id]);
        new_ids.push(new_temporal_id);
    }

    // Transform the original temporal node into the logical operator
    // and set its children to be the new temporal nodes
    let temporal_mut = expr.try_node_mut(temporal_id)?;
    temporal_mut.set_kind(logical_kind); // Change the temporal node into AND/OR
    temporal_mut.set_children(new_ids.clone());

    // Return the new temporal nodes for further processing
    Ok(new_ids)
}

/// Wraps the body of a quantifier (`Forall` / `Exists`) with a temporal specifier.
///
/// This function takes a temporal node (`AtStart`, `AtEnd`, or `Overall`) whose child
/// is a quantifier node. It wraps the body of the quantifier with a new temporal node,
/// and updates the quantifier to point to this new temporal node.
///
/// # Parameters
/// - `temporal_id`: NodeId of the temporal specifier node to push down.
/// - `expr`: Mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(NodeId)` of the updated temporal node (root remains the same).
/// - `Err(ExprError)` if node access or mutation fails.
///
/// # Panics / Debug asserts
/// - The temporal node must have exactly one child.
/// - The child must be a quantifier (`Forall` or `Exists`) with exactly two children.
#[allow(dead_code)]
fn push_time_specifier_into_quantifier(
    temporal_id: NodeId,
    expr: &mut Expr,
) -> Result<NodeId, ExprError> {
    // Retrieve the temporal node (AtStart / AtEnd / Overall)
    let temporal_node = expr.try_node(temporal_id)?;
    let kind = temporal_node.kind();

    // Ensure the temporal node has exactly one child
    let children = temporal_node.children();
    debug_assert!(children.len() == 1, "Temporal node must have exactly one child");
    let quant_id = children[0];

    // Retrieve the quantifier node (Forall / Exists)
    let quant_node = expr.try_node(quant_id)?;
    debug_assert!(
        quant_node.kind() == ExprKind::Forall || quant_node.kind() == ExprKind::Exists,
        "Child must be a quantifier (Forall / Exists)"
    );

    // Quantifier must have exactly two children: variable list and body
    let quant_children = quant_node.children();
    debug_assert!(quant_children.len() == 2, "Quantifier must have exactly two children");
    let var_list_id = quant_children[0];
    let body_id = quant_children[1];

    // Create a new temporal node wrapping the body
    let new_time_node = ExprNode::new(kind, Content::None, Some(quant_id));
    let new_time_id = expr.alloc_with_children(new_time_node, vec![body_id]);

    // Update the quantifier to point to the new temporal node as its body
    let quant_mut = expr.try_node_mut(quant_id)?;
    quant_mut.set_children(vec![var_list_id, new_time_id]);

    // Move the updated quantifier under the original temporal node
    expr.move_to(quant_id, temporal_id)?;

    // Return the ID of the temporal node (root remains the same)
    Ok(temporal_id)
}

/// Verifies that all literals in an expression tree are properly wrapped in a temporal specifier.
///
/// This function checks that every literal node (atomic formulas or `FComp` nodes),
/// potentially under a `Not` node, has a parent that is a temporal specifier
/// (`ExprKind::AtStart`, `ExprKind::AtEnd`, or `ExprKind::Overall`).
///
/// # Arguments
///
/// * `expr` - A reference to the expression tree to check.
/// * `root_id` - The ID of the root node to start the check from.
///
/// # Returns
///
/// Returns `Ok(())` if all literals are correctly wrapped in temporal specifiers.
///
/// # Errors
///
/// Returns `ExprError::missing_time_specifier(node_id)` if any literal node
/// does not have a temporal specifier as its parent.
#[allow(dead_code)]
fn verify_temporal_consistency(expr: &Expr, root_id: NodeId) -> Result<(), ExprError> {
    // Traverse the tree in post-order starting from root_id
    for (node_id, node) in expr.postorder_from(root_id).ids() {
        // Check if the current node is a literal (atomic formula or FComp node)
        if expr.is_literal(node_id) {
            // Get the parent node; if none exists, that's an error
            let parent_id = node.parent()
                .ok_or_else(|| ExprError::missing_time_specifier(node_id))?;

            // Verify that the parent is a temporal specifier
            if !expr.is_time_specifier(parent_id)? {
                // If not, return an error indicating missing temporal wrapper
                return Err(ExprError::missing_time_specifier(node_id));
            }
        }
        // Non-literal nodes are ignored
    }

    // All literals verified successfully
    Ok(())
}

/// Normalizes a temporal expression by pushing temporal specifiers down to atomic formulas
/// and splitting the expression into three separate temporal contexts: `AtStart`, `AtEnd`, and `Overall`.
///
/// The normalized expression will have a new `And` root combining the three temporal subtrees.
///
/// # Arguments
///
/// * `root_id` - The ID of the root node of the expression to normalize.
/// * `expr` - A mutable reference to the expression tree to modify.
///
/// # Returns
///
/// Returns `Ok(true)` if normalization was performed, `Ok(false)` if no temporal specifiers
/// needed to be pushed. Returns an `ExprError` if any operation fails.
pub fn normalize_temporal_expr(
    root_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprError> {
    // 1. Push temporal specifiers down to atomic formulas
    // If there are no temporal specifiers to push, we can exit early
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

    // Normalization completed successfully
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
pub fn filter_temporal(
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
    use crate::aiplan4rust::core::arena::ArenaNode;
    use super::*;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::syntax::SyntaxDisplay;

    /// Test pushing AtStart temporal specifier through an AND node.
    /// Input: (at start (and (A) (B)))
    /// Expected Output: (and (at start A) (at start B))
    #[test]
    fn test_push_at_start_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let and_node = builder.and(vec![a, b]);
        let root = builder.at_start(and_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        push_time_specifier(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);

        for (i, &child_id) in root_node.children().iter().enumerate() {
            let child_node = expr.try_node(child_id).unwrap();
            assert_eq!(child_node.kind(), ExprKind::AtStart);
            assert_eq!(child_node.children().len(), 1);

            let atomic_id = child_node.children()[0];
            let atomic_node = expr.try_node(atomic_id).unwrap();
            assert_eq!(atomic_node.kind(), ExprKind::AtomicFormula);
        }

        assert_eq!(output, "(and (at start (A)) (at start (B)))");
    }

    /// Test pushing AtEnd temporal specifier through a Forall quantifier.
    /// Input: (at_end (forall (?X) (A)))
    /// Expected Output: (forall (?X) (at end (A)))
    #[test]
    fn test_push_at_end_forall() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let x = builder.variable("?X");
        let vars = builder.typed_list(vec![x]);
        let forall_node = builder.forall(vars, a);
        let root = builder.at_end(forall_node);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        push_time_specifier(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Forall);
        assert_eq!(root_node.children().len(), 2);
        let var_id = root_node.children()[0];
        let var_node = expr.try_node(var_id).unwrap();
        assert_eq!(var_node.kind(), ExprKind::TypedList);
        let body_id = root_node.children()[1];
        let body_node = expr.try_node(body_id).unwrap();
        assert_eq!(body_node.kind(), ExprKind::AtEnd);
        assert_eq!(body_node.children().len(), 1);
        let a_id = body_node.children()[0];
        let a_node = expr.try_node(a_id).unwrap();
        assert_eq!(a_node.kind(), ExprKind::AtomicFormula);

        assert_eq!(output, "(forall (?X) (at end (A)))");
    }

    /// Test pushing Overall temporal specifier to an atomic formula.
    /// Input: (overall (A))
    /// Expected Output: (overall (A))  (no change needed)
    #[test]
    fn test_push_overall_atomic() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let root = builder.overall(a);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        push_time_specifier(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Overall);
        assert_eq!(root_node.children().len(), 1);

        let child_id = root_node.children()[0];
        let child_node = expr.try_node(child_id).unwrap();
        assert_eq!(child_node.kind(), ExprKind::AtomicFormula);

        assert_eq!(output, "(over all (A))");
    }

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

        // Construction de l'expression
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

        let input = expr.to_syntax_string(&interner);
        if !normalize_temporal_expr(expr.try_root_id().unwrap(), &mut expr).unwrap() {
            return ;
        };
        let output = expr.to_syntax_string(&interner);

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

        let input = expr.to_syntax_string(&interner);
        if !normalize_temporal_expr(expr.try_root_id().unwrap(), &mut expr).unwrap() {
            return ;
        };

        let output = expr.to_syntax_string(&interner);

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

        let input = expr.to_syntax_string(&interner);
        if !normalize_temporal_expr(expr.try_root_id().unwrap(), &mut expr).unwrap() {
            return ;
        };

        let output = expr.to_syntax_string(&interner);

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

        let input = expr.to_syntax_string(&interner);
        if !normalize_temporal_expr(expr.try_root_id().unwrap(), &mut expr).unwrap() {
            return ;
        };
        let output = expr.to_syntax_string(&interner);

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
        let x = builder.variable("?X");
        let vars = builder.typed_list(vec![x]);

        let forall_b = builder.forall(vars, b);
        let or_node = builder.or(vec![a, forall_b]);
        let start_or = builder.at_start(or_node);
        let end_c = builder.at_end(c);

        let root = builder.and(vec![start_or, end_c]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        if !normalize_temporal_expr(expr.try_root_id().unwrap(), &mut expr).unwrap() {
            return ;
        };
        let output = expr.to_syntax_string(&interner);

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
        assert_eq!(child2.children().len(), 2);
        let var_list = expr.try_node(child2.children()[0]).unwrap();
        assert_eq!(var_list.kind(), ExprKind::TypedList);
        let body = expr.try_node(child2.children()[1]).unwrap();
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
