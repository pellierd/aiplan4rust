use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind, ExprNode};
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
/// # Transformations applied
/// - `(AtStart X)` → temporal specifier pushed down to children of `X`.
/// - `(AtEnd X)` → temporal specifier pushed down similarly.
/// - `(Overall X)` → temporal specifier pushed down similarly.
/// - Handles logical operators (`And`, `Or`) by duplicating the temporal specifier for each child.
/// - Handles quantifiers (`Forall`, `Exists`) by wrapping the body in a temporal node.
///
/// # Preconditions
/// - Must be called **after** `push_negation` to ensure all literals under `Not` nodes
///   are in a form suitable for temporal propagation.
/// - Part of the normalization pipeline orchestrated by the `normalize` module.
///   Users should not call this directly; they should use the higher-level normalizer
///   functions to guarantee the correct order.
///
/// # Supported child node kinds
/// Only the following kinds of children under a temporal specifier are allowed:
/// - `And`, `Or` (logical operators, De Morgan rules apply)
/// - `Forall`, `Exists` (quantifiers, wrapped with temporal node)
/// - `Not`, `FComp`, `AtomicFormula` (leaves, propagation stops here)
///
/// Any other kind of child node will trigger an **error** (`ExprError::invalid_expr_node`).
///
/// # Parameters
/// - `root_id`: NodeId of the root of the subtree to process.
/// - `expr`: Mutable reference to the expression tree.
///
/// # Returns
/// - `Ok(true)` if any modifications were made (new temporal nodes inserted).
/// - `Ok(false)` if the tree was already in the desired form.
/// - `Err(ExprError::invalid_expr_node)` if a temporal specifier has a child of unsupported kind.
/// - `Err(ExprError)` if accessing nodes fails.
///
/// # Notes
/// - Mutates the tree in place.
/// - Uses a stack for depth-first traversal to handle temporal specifiers.
/// - Newly created temporal nodes are pushed onto the stack for further processing.
/// - Assumes each temporal specifier node has exactly one child.
///   If this invariant is violated, a `MalformedExprNode` or similar error should be raised
///   to indicate an IR structural problem.
/// - After this step, all atomic formulas are guaranteed to be wrapped in a temporal specifier,
///   making them ready for further normalization or factorization.
///
/// # Example usage
/// ```rust
/// // Part of the normalization pipeline managed by the `normalize` module
/// push_time_specifier(root_id, &mut expr)?;
/// ```
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
                ExprKind::Not | ExprKind::FComp | ExprKind::AtomicFormula => {
                    continue;
                }
                _ => {
                    return Err(ExprError::invalid_expr_node(child_id, child.kind()));
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

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::core::arena::ArenaNode;
    use super::*;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::syntax::SyntaxInternerDisplay;

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

        let input = expr.to_syntax_string_with_interner(&interner);
        push_time_specifier(root, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);

        for &child_id in root_node.children().iter() {
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

        let input = expr.to_syntax_string_with_interner(&interner);
        push_time_specifier(root, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

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

        let input = expr.to_syntax_string_with_interner(&interner);
        push_time_specifier(root, &mut expr).unwrap();
        let output = expr.to_syntax_string_with_interner(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Overall);
        assert_eq!(root_node.children().len(), 1);

        let child_id = root_node.children()[0];
        let child_node = expr.try_node(child_id).unwrap();
        assert_eq!(child_node.kind(), ExprKind::AtomicFormula);

        assert_eq!(output, "(over all (A))");
    }
}
