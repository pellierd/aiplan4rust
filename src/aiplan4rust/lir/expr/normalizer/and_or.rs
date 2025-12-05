use std::collections::HashSet;
use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::syntax::tree::NodeId;
use crate::aiplan4rust::syntax::display::SyntaxDisplay;

/// Simplifies an AND or OR node in a PDDL expression tree, including merging WHEN expressions.
///
/// This function performs several simplifications on a node of type `AND` or `OR`,
/// processing the node in place. It does nothing if the node is of another kind.
///
/// The simplifications are performed in the following order:
/// 1. **Flatten nested nodes**: If the node has children of the same kind, they are
///    lifted up to the current node. For example:
///    `(and A (and B C))` becomes `(and A B C)`.
/// 2. **Canonical ordering of children**: Children of the node are sorted in a
///    canonical order. This ensures that structural comparisons and deduplication
///    are order-independent. Only AND/OR nodes are affected; other nodes are skipped.
/// 3. **Structural deduplication**: Duplicate subtrees are removed using a
///    structural hash (`sub_expr_hash`). For example:
///    `(and (and A B) (and A B))` becomes `(and (and A B))`.
/// 4. **Merge WHEN expressions**:
///    - All `When` expressions among the children are grouped by their effect.
///    - Conditions with the same effect are merged; if multiple conditions exist,
///      they are combined under a new `Or` node.
///    - Non-`When` children remain unchanged.
///    - This step may modify the children, and the node will reflect merged `When`s.
/// 5. **Tautology and contradiction elimination**:
///    - For OR nodes: `(or A (not A))` → (and)
///    - For AND nodes: `(and A (not A))` → (or)
/// 6. **Single-child reduction**: If the node has only one child after deduplication,
///    it is replaced by that child. For example:
///    `(and A)` becomes `A`.
/// 7. **Empty-node simplification**: If the node has no children, it is replaced
///    by a neutral value (true for `AND`, false for `OR` depending on your semantics).
///
/// # Parameters
/// - `node_id`: The ID of the node to simplify.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if the simplification succeeds.
/// - `Err(ExprError)` if accessing a node fails.
///
/// # Notes
/// - This function assumes that `expr` is a well-formed tree and that `node_id` exists.
/// - Only AND or OR nodes are simplified; other nodes are skipped silently.
/// - The `merge_when_in_place` step ensures that logically equivalent WHEN conditions
///   are grouped and avoids redundant branches in the expression.
/// - Sorting children in canonical order ensures that `deep_subexpr_eq` and other
///   structural equality checks behave consistently regardless of original child order.
///
/// # Example
/// ```ignore
/// // Suppose expr represents: (and A (and B C) (when X Y) (when Z Y))
/// let node_id = expr.root_id().unwrap();
/// normalize(node_id, &mut expr)?;
/// // After simplification, the expression becomes: (and A B C (when (or X Z) Y))
/// ```
pub(super) fn normalize(
    node_id: NodeId,
    expr: &mut Expr
) -> Result<(), ExprError> {

    // Step 1: Flatten nested AND/OR nodes of the same kind
    flatten_and_or_node(node_id, expr)?;

    // Step 2: Sort children in canonical order
    canonicalize_and_or_node(node_id, expr)?;

    // Step 3: Deduplicate structurally
    deduplicate_and_or_node(node_id, expr)?;

    // Step 4: Merge WHEN expressions
    merge_when(node_id, expr)?; // merge WHEN expressions grouped by effect

    // Step 5: Simplify tautologies and contradictions
    if simplify_tautologies_and_contradictions(node_id, expr)? {
        return Ok(());
    }

    // Step 6: Reduce nodes with a single child
    if reduce_single_and_or_node(node_id, expr)? {
        return Ok(());
    }

    // Step 7: Simplify empty nodes
    simplify_empty_and_or_node(node_id, expr)?;

    Ok(())
}


/// Flattens nested AND/OR nodes of the same kind into a single node.
///
/// For example:
/// ```text
/// (and A (and B C) D)  ->  (and A B C D)
/// (or X (or Y Z))      ->  (or X Y Z)
/// ```
///
/// # Parameters
/// - `node_id`: The ID of the node to flatten. Only AND/OR nodes are affected.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Behavior
/// - If `node_id` corresponds to an AND or OR node, any child nodes of the same kind
///   are "lifted" so that their children become direct children of `node_id`.
/// - Children of a different kind are preserved as-is.
/// - The original order of children is maintained.
/// - If `node_id` is not an AND or OR node, the function does nothing and returns `Ok(false)`.
///
/// # Returns
/// - `Ok(true)` if the node's structure was modified (children added, removed, or flattened).
/// - `Ok(false)` if the node was not an AND/OR or no structural change occurred.
/// - `Err(ExprError)` if accessing any node fails.
///
/// # Notes
/// - Uses `std::mem::take` to temporarily take ownership of children vectors, avoiding
///   borrow checker conflicts.
/// - Useful for simplifying logical expressions in PDDL-like ASTs by reducing unnecessary nesting.
#[allow(dead_code)]
fn flatten_and_or_node(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    // Borrow the node immutably to check its kind.
    let kind = expr.try_node(node_id)?.kind();

    // Only AND or OR nodes are flattened; skip other node types.
    debug_assert!(kind == ExprKind::And || kind == ExprKind::Or);

    // Take ownership of the current children vector to avoid borrow conflicts.
    let node = expr.try_node_mut(node_id)?;
    let children_len = node.children().len(); // length before flattening
    let children = std::mem::take(node.children_mut());

    // Prepare a new vector to hold the flattened children.
    let mut flat = Vec::new();
    let mut modified = false;

    // Iterate over each child of the node.
    for child_id in children {
        // Borrow the child immutably to check its kind.
        let child_kind = expr.try_node(child_id)?.kind();

        // If the child is of the same kind as the parent (nested AND/OR),
        // take ownership of the child's children and append them to `flat`.
        if child_kind == kind {
            flat.extend(std::mem::take(expr.try_node_mut(child_id)?.children_mut()));
            modified = true;
        } else {
            // Otherwise, keep the child as-is.
            flat.push(child_id);
        }
    }

    // If the total number of children changed, mark as modified.
    if flat.len() != children_len {
        modified = true;
    }

    // Assign the new flattened vector back to the node.
    expr.try_node_mut(node_id)?.set_children(flat);

    Ok(modified)
}

/// Canonicalizes the children of an AND/OR node.
///
/// This function sorts the children of a commutative node (`AND` or `OR`) into a canonical order,
/// which ensures that structural equality checks and deduplication are independent of the original
/// order of the children.
///
/// # Parameters
/// - `node_id`: The ID of the node whose children should be canonicalized. Only AND/OR nodes are affected.
/// - `expr`: A mutable reference to the expression tree.
///
/// # Behavior
/// - For AND/OR nodes, children are sorted in ascending order of `NodeId` (or any other canonical key).
/// - Non-AND/OR nodes are silently skipped (no error is returned).
///
/// # Returns
/// - `Ok(())` if the operation succeeds.
/// - `Err(ExprError)` if accessing the node fails.
///
/// # Notes
/// - Sorting is done in-place by mutably borrowing the children vector.
/// - Canonical ordering allows functions like `deep_subexpr_eq` and deduplication to work
///   consistently regardless of the input order.
///
/// # Example
/// ```ignore
/// // Before canonicalization: AND node with children [3, 1, 2]
/// // After calling `canonicalize_and_or_children(node_id, &mut expr)`:
/// // Children are [1, 2, 3]
/// ```
fn canonicalize_and_or_node(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    let kind = expr.try_node(node_id)?.kind();
    if kind != ExprKind::And && kind != ExprKind::Or {
        return Ok(());
    }

    let node = expr.try_node_mut(node_id)?;
    node.children_mut().sort();

    Ok(())
}

/// Removes duplicate children from an AND or OR node in an expression tree.
///
/// # Parameters
/// - `node_id`: The ID of the AND/OR node to deduplicate.
/// - `expr`: Mutable reference to the expression tree.
///
/// # Behavior
/// - Iterates over the children of the node and keeps only the first occurrence of each child.
/// - Duplicates are detected based on **structural equality** via `deep_sub_expr_eq`.
/// - Only AND and OR nodes are processed; other node types are skipped silently.
/// - The original order of children is preserved.
///
/// # Returns
/// - `Ok(true)` if the node was deduplicated (i.e., at least one duplicate was removed).
/// - `Ok(false)` if no deduplication was necessary or the node is not AND/OR.
/// - `Err(ExprError)` if accessing a node fails.
///
/// # Notes
/// - Deduplication uses a nested loop to check structural equality, not just `NodeId`.
/// - Mutably borrows the node only once to write back the deduplicated children.
/// - Does **not** sort the children; original order is preserved.
///
/// # Example
/// ```ignore
/// // AND node with duplicate NodeIds: (and A A B)
/// // After deduplication: (and A B)
/// ```
#[allow(dead_code)]
fn deduplicate_and_or_node(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    // Borrow the node immutably
    let node = expr.try_node(node_id)?;

    // Only process AND or OR nodes
    let kind = node.kind();
    debug_assert!(kind == ExprKind::And || kind == ExprKind::Or);

    // Prepare a vector for deduplicated children
    let mut deduped = Vec::with_capacity(node.children().len());

    for &child_id in node.children() {
        // Check if child is already present structurally
        let mut is_duplicate = false;
        for &seen_id in &deduped {
            if expr.deep_sub_expr_eq(seen_id, child_id)? {
                is_duplicate = true;
                break;
            }
        }
        if !is_duplicate {
            deduped.push(child_id);
        }
    }

    // Write back the deduplicated children
    expr.try_node_mut(node_id)?.set_children(deduped);

    Ok(true)
}

/// Checks for tautologies and contradictions in an AND/OR node.
///
/// - For OR nodes: `(or φ (not φ))` → true (represented as an empty AND)
/// - For AND nodes: `(and φ (not φ))` → false (represented as an empty OR)
///
/// # Parameters
/// - `node_id`: NodeId of the AND/OR node to simplify
/// - `expr`: Mutable reference to the expression tree
///
/// # Returns
/// - `Ok(true)` if the node has been replaced with a neutral value (true/false)
/// - `Ok(false)` if no simplification was performed
/// - `Err(ExprError)` if any operation fails
///
/// # Steps
/// 1. Verify that the node is an AND or OR. Skip otherwise.
/// 2. Collect all direct children and track which ones are negated.
/// 3. Check for intersection between the sets of positive and negated children:
///    - For OR: if any φ and (not φ) exist, the OR is always true → replace with empty AND
///    - For AND: if any φ and (not φ) exist, the AND is always false → replace with empty OR
/// 4. Update the node kind and clear children if a tautology/contradiction is found.
/// 5. Return whether the node was simplified.
pub fn simplify_tautologies_and_contradictions(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprError> {
    // Borrow the node
    let node = expr.try_node(node_id)?;
    let kind = node.kind();

    // Only AND/OR nodes are relevant
    debug_assert!(kind == ExprKind::And || kind == ExprKind::Or);

    let children = node.children();

    // Track regular and negated children
    let mut child_set = HashSet::new();
    let mut negated_set = HashSet::new();

    for &child_id in children {
        let child = expr.try_node(child_id)?;
        match child.kind() {
            ExprKind::Not => {
                // Add the inner child of Not node
                let not_child_id = child.children()[0];
                negated_set.insert(not_child_id);
            }
            _ => {
                child_set.insert(child_id);
            }
        }
    }

    let node_mut = expr.try_node_mut(node_id)?;

    match kind {
        ExprKind::Or => {
            // If there exists a child φ and its negation, the OR is a tautology → true
            if !child_set.is_disjoint(&negated_set) {
                node_mut.set_kind(ExprKind::And); // represents true
                node_mut.set_children(vec![]);
                return Ok(true);
            }
        }
        ExprKind::And => {
            // If there exists a child φ and its negation, the AND is a contradiction → false
            if !child_set.is_disjoint(&negated_set) {
                node_mut.set_kind(ExprKind::Or); // represents false
                node_mut.set_children(vec![]);
                return Ok(true);
            }
        }
        _ => {}
    }

    // No simplification applied
    Ok(false)
}

/// Reduces an AND/OR node that has exactly one child.
///
/// # Parameters
/// - `node_id`: the `NodeId` of the node to reduce. Only AND/OR nodes are considered.
/// - `expr`: mutable reference to the expression tree.
///
/// # Behavior
/// - If the node is an AND or OR and has exactly one child:
///     - If the node has a parent, replace the node in the parent's children list with its single child.
///     - If the node is the root, set the single child as the new root.
/// - If the node has zero or more than one child, or is not an AND/OR, the function does nothing.
///
/// # Returns
/// - `Ok(true)` if the node was actually reduced (i.e., replaced by its single child).
/// - `Ok(false)` if no reduction was performed (zero or multiple children, or node not AND/OR).
/// - `Err(ExprError)` if accessing a node or the root fails.
///
/// # Notes
/// - Intended to be called as part of the simplification pipeline on AND/OR nodes only.
/// - Reductions are safe and preserve the logical meaning of the expression.
#[allow(dead_code)]
fn reduce_single_and_or_node(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprError> {
    // Borrow the node immutably to check its kind
    let node = expr.try_node(node_id)?;

    // Only AND/OR nodes are considered
    let kind = node.kind();
    debug_assert!(kind == ExprKind::And || kind == ExprKind::Or);

    let children = node.children();
    // Only reduce if there is exactly one child
    if children.len() != 1 {
        return Ok(false);
    }

    let single_child = children[0];

    // Check if the node has a parent
    if let Some(parent_id) = node.parent() {
        // Mutably borrow the parent
        let parent = expr.try_node_mut(parent_id)?;
        // Find the position of node_id in parent's children and replace it
        if let Some(pos) = parent.children().iter().position(|&id| id == node_id) {
            parent.children_mut()[pos] = single_child;
        }
    } else {
        // Node is root → update the root ID
        expr.set_root_id(single_child)?;
    }

    Ok(true)
}

/// Simplifies an AND/OR node in a PDDL expression according to standard PDDL semantics.
///
/// # Parameters
/// - `node_id`: the `NodeId` of the node to simplify. Only AND/OR nodes are affected.
/// - `expr`: mutable reference to the `Expr` tree, used to access and mutate child nodes.
///
/// # Behavior
/// The function handles three main cases for AND/OR nodes:
///
/// 1. **Absorbing child**: If a child is an empty node of the **opposite type**, the parent node
///    is replaced by the child's kind and its children are cleared.
///    - Example: `(and (or))` → becomes `(or)`
///    - Semantically, `(or)` with no children evaluates to `false`, `(and)` with no children evaluates to `true`.
///
/// 2. **Neutral child**: If a child is an empty node of the **same type**, it is ignored and
///    not included in the simplified children list.
///    - Example: `(and (and))` → becomes `(and)` (still true)
///
/// 3. **Child kept**: All other children are preserved in the simplified node.
///
/// # Returns
/// - `Ok(true)` if the node was modified (simplified or children changed).
/// - `Ok(false)` if no changes were made to the node.
/// - `Err(ExprError)` if accessing a child node fails.
///
/// # Notes
/// - Copies only the `NodeId`s of children (lightweight integers).
/// - Temporarily takes ownership of the children vector via `std::mem::take` for safe mutable operations.
/// - Preserves PDDL semantics:
///   - `(and)` with no children → `true`
///   - `(or)` with no children → `false`
///   - No explicit `true` or `false` constants are introduced.
#[allow(dead_code)]
fn simplify_empty_and_or_node(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprError> {
    // 1. Access the node corresponding to node_id (immutable borrow)
    let node = expr.try_node(node_id)?;
    let node_kind = node.kind(); // get the node type: And / Or / other

    // 2. Ensure this node is AND or OR
    debug_assert!(node_kind == ExprKind::And || node_kind == ExprKind::Or);

    // 3. Prepare a new vector to store the children that will remain
    //    Only copies NodeIds (integers), which is lightweight
    let mut new_children = Vec::with_capacity(node.children().len());
    let mut simplified = false;

    // 4. Iterate over all children of the node
    for &child_id in node.children() {
        let child = expr.try_node(child_id)?; // immutable borrow of the child
        let child_kind = child.kind();        // child's type
        let child_empty = child.children().is_empty(); // check if child is empty

        // -------- Case 1: absorbing child --------
        // If the child is empty and of the opposite type:
        //   - (and (or)) → becomes (or) → false
        //   - (or (and)) → becomes (and) → true
        if (node_kind == ExprKind::And && child.is_empty_or())
            || (node_kind == ExprKind::Or && child.is_empty_and())
        {
            // Mutably borrow the parent node only here
            let node_mut = expr.try_node_mut(node_id)?;
            node_mut.set_kind(child_kind);  // replace the node type
            node_mut.set_children(vec![]);  // clear all children
            return Ok(true);                  // simplification done
        }

        // -------- Case 2: neutral child --------
        // If the child is empty and of the same type:
        //   - (and (and)) → ignore the child → remains (and) → true
        //   - (or (or))   → ignore the child → remains (or)  → false
        if child_kind == node_kind && child_empty {
            simplified = true;
            continue; // skip this child
        }

        // -------- Case 3: keep child --------
        // All other children are preserved
        new_children.push(child_id);
    }

    // -------- Case 3: update node children at the end --------
    // Mutably borrow the parent node once
    if new_children.len() != node.children().len() || simplified {
        let node_mut = expr.try_node_mut(node_id)?;
        node_mut.set_children(new_children);
        simplified = true;
    }

    Ok(simplified)
}

/// Main function that merges `When` expressions under an `And` or `Or` node
/// and updates the node in-place.
///
/// This function collects all `When` children, merges conditions that share
/// the same effect, rebuilds the node's children with merged `When`s, and
/// returns a boolean indicating whether any fusion (i.e., multiple conditions
/// merged into an `Or`) occurred.
///
/// # Arguments
///
/// * `node_id` - The ID of the parent node (`And` or `Or`) whose children will be updated.
/// * `expr` - Mutable reference to the `Expr` tree being updated.
///
/// # Returns
///
/// * `Ok(true)` if any fusion occurred (an `Or` was created).
/// * `Ok(false)` if no fusion occurred (all `When`s had a single condition).
/// * `Err(ExprError)` if an error occurs during collection, allocation, or normalization.
///
/// # Behavior
///
/// 1. Collects non-`When` children and merges `When` conditions by effect using `collect_and_merge_when`.
/// 2. Rebuilds the node's children using `rebuild_children_with_merged_when`, which now returns a boolean.
/// 3. Returns whether any fusion occurred.
///
/// # Example
///
/// ```ignore
/// let fusion_occurred = merge_when_in_place(node_id, &mut expr)?;
/// if fusion_occurred {
///     println!("Some WHEN conditions were merged into OR nodes");
/// }
/// ```
pub fn merge_when(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprError> {
    // Collect non-WHEN children and merged WHEN conditions
    let (non_when, merged_map) = collect_and_merge_when(node_id, expr)?;

    // Rebuild the node's children and get whether a fusion occurred
    let fusion_occurred = rebuild_children_with_merged_when(node_id, non_when, merged_map, expr)?;

    // Return the fusion flag
    Ok(fusion_occurred)
}

/// Iterates over the children of a node, collects non-`When` children,
/// and merges `When` expressions by their effect.
///
/// This function processes a logical `And` or `Or` node. It separates out the children
/// that are **not** `When` expressions, and merges all `When` expressions that have
/// the same effect. If multiple conditions share the same effect, they are grouped together.
///
/// # Arguments
///
/// * `node_id` - The ID of the parent node (`And` or `Or`) whose children are being processed.
/// * `expr` - A reference to the expression tree containing the node.
///
/// # Returns
///
/// * `Ok((non_when, merged_when))` where:
///     - `non_when` is a vector of NodeIds for children that are not `When` expressions.
///     - `merged_when` is a vector of tuples `(effect_node, conditions)`:
///         - `effect_node` is the NodeId of the effect of the `When`.
///         - `conditions` is a vector of NodeIds representing all conditions associated with that effect.
/// * `Err(ExprError)` if accessing the expression tree fails.
///
/// # Behavior
///
/// 1. Retrieve the node corresponding to `node_id` and assert it is an `And` or `Or`.
/// 2. Initialize empty vectors:
///     - `non_when` for children that are not `When`.
///     - `merged_map` for grouping conditions by their effect.
/// 3. Iterate over each child of the node:
///     - If the child is not a `When`, add it to `non_when`.
///     - If the child is a `When`:
///         - Extract its condition (`cond_id`) and effect (`eff_id`).
///         - Search `merged_map` for an existing entry with the same effect using `deep_sub_expr_eq`.
///             - If found, append the condition to the existing list.
///             - If not found, create a new entry `(eff_id, vec![cond_id])`.
/// 4. Return `non_when` and `merged_map`.
///
/// # Notes
///
/// - `deep_sub_expr_eq` is used to compare effects structurally. Later, this could be optimized
///   using precomputed hashes for faster comparisons.
/// - The function preserves the original order of non-`When` nodes.
///
/// # Example
///
/// ```ignore
/// // Suppose node_id is an AND node containing WHEN expressions
/// let (non_when, merged_when) = collect_and_merge_when(node_id, &expr)?;
/// // non_when contains all non-WHEN children
/// // merged_when groups WHEN conditions by effect
/// ```
fn collect_and_merge_when(
    node_id: NodeId,
    expr: &Expr
) -> Result<(Vec<NodeId>, Vec<(NodeId, Vec<NodeId>)>), ExprError> {
    // Retrieve the node and ensure it is an AND or OR
    let node = expr.try_node(node_id)?;
    debug_assert!(
        matches!(node.kind(), ExprKind::And | ExprKind::Or),
        "rebuild_children_with_merged_when expects an AND or OR node"
    );

    // Vector to store non-WHEN children
    let mut non_when: Vec<NodeId> = Vec::new();
    // Vector to group conditions by their effect
    let mut merged_map: Vec<(NodeId, Vec<NodeId>)> = Vec::new();

    // Iterate over all children
    for &child_id in node.children() {
        let child = expr.try_node(child_id)?;
        if child.kind() != ExprKind::When {
            // Non-WHEN children go directly into the non_when vector
            non_when.push(child_id);
        } else {
            // Ensure the WHEN node has exactly two children: condition and effect
            debug_assert!(
                child.children().len() == 2,
                "WHEN node must have exactly two children: condition and effect"
            );

            // Extract condition and effect
            let cond_id = child.children()[0];
            let eff_id = child.children()[1];

            // Merge conditions by effect
            let mut found = false;
            for (existing_eff_id, conds) in &mut merged_map {
                if expr.deep_sub_expr_eq(*existing_eff_id, eff_id)? {
                    conds.push(cond_id);
                    found = true;
                    break;
                }
            }

            // If no existing entry with this effect, create a new one
            if !found {
                merged_map.push((eff_id, vec![cond_id]));
            }
        }
    }

    Ok((non_when, merged_map))
}

/// Rebuilds the children of a logical `And` or `Or` node by merging `When` expressions.
/// Returns `true` if any fusion occurred (i.e., if multiple conditions were combined into an `Or`).
///
/// This function replaces `When` nodes with merged versions, combining conditions
/// that share the same effect. Non-`When` children are preserved as-is.
///
/// # Arguments
///
/// * `node_id` - The ID of the parent node (`And` or `Or`) whose children will be updated.
/// * `non_when` - A vector of NodeIds representing children that are **not** `When` expressions.
/// * `merged_when` - A vector of tuples `(effect_node, conditions)` representing merged `When` expressions.
///                   Each tuple contains the effect node ID and a vector of condition node IDs.
/// * `expr` - Mutable reference to the `Expr` tree being updated.
///
/// # Returns
///
/// * `Ok(true)` if a fusion occurred (an `Or` node was created for multiple conditions).
/// * `Ok(false)` if no fusion occurred (all `When` nodes had a single condition).
/// * `Err(ExprError)` if an error occurs during node allocation or normalization.
///
/// # Behavior
///
/// 1. Retrieves the node and asserts it is an `And` or `Or`.
/// 2. Pre-allocates a new vector for children, including space for `non_when` and merged `When` nodes.
/// 3. Copies all `non_when` nodes into the new vector.
/// 4. Iterates over each `(effect, conditions)` in `merged_when`:
///     - If there is only one condition, use it directly.
///     - If multiple conditions exist, create an `Or` node and normalize it, marking `fusion_occurred`.
///     - Allocate a `When` node with the condition (or `Or`) and the effect.
///     - Append the `When` node to the new children vector.
/// 5. Replaces the children of `node_id` with the new vector.
/// 6. Returns whether any fusion occurred.
///
/// # Notes
///
/// - Assumes `node_id` is an `And` or `Or`.
/// - `normalize` is applied only to `Or` nodes with multiple conditions.
///
/// # Example
///
/// ```ignore
/// let fusion = rebuild_children_with_merged_when(node_id, non_when_vec, merged_when_vec, &mut expr)?;
/// if fusion {
///     println!("Some WHEN conditions were merged into OR nodes");
/// }
/// ```
fn rebuild_children_with_merged_when(
    node_id: NodeId,
    non_when: Vec<NodeId>,
    merged_when: Vec<(NodeId, Vec<NodeId>)>,
    expr: &mut Expr,
) -> Result<bool, ExprError> {
    // Retrieve the node and assert it is an AND or OR
    let node = expr.try_node(node_id)?;
    debug_assert!(
        matches!(node.kind(), ExprKind::And | ExprKind::Or),
        "rebuild_children_with_merged_when expects an AND or OR node"
    );

    // Pre-allocate the new children vector with enough space
    let mut new_children = Vec::with_capacity(non_when.len() + merged_when.len());
    new_children.extend(non_when); // Copy non-WHEN nodes directly

    // Flag to indicate if any fusion occurred
    let mut fusion_occurred = false;

    // Iterate over merged WHEN tuples (effect, conditions)
    for (eff_id, conds) in merged_when {
        // Determine the condition node
        let cond_node = if conds.len() == 1 {
            // Single condition, use as-is
            conds[0]
        } else {
            // Multiple conditions, create OR node → fusion
            fusion_occurred = true;
            let or_node = expr.alloc_with_children(
                ExprNode::new(ExprKind::Or, Content::None, None),
                conds,
            );
            normalize(or_node, expr)?; // Simplify OR node
            or_node
        };

        // Create the WHEN node combining the condition(s) and effect
        let when_node = expr.alloc_with_children(
            ExprNode::new(ExprKind::When, Content::None, None),
            vec![cond_node, eff_id],
        );

        // Append the WHEN node to the new children vector
        new_children.push(when_node);
    }

    // Update the original node with the new children
    expr.try_node_mut(node_id)?.set_children(new_children);

    // Return whether a fusion occurred
    Ok(fusion_occurred)
}

#[cfg(test)]
mod realistic_tests {
    use super::*;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;

    /// Realistic test: AND root with nested AND/OR children, duplicates, and empty OR
    ///
    /// Input: (and (A) (and (B) (C) (B)) (or) (and (C)))
    /// Expected: (or) <- empty OR inside AND makes the root OR
    #[test]
    fn test_realistic_and_flatten() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let inner1 = builder.and(vec![b, c, b]); // duplicated B
        let inner2 = builder.and(vec![c]);       // single C
        let empty_or = builder.or(vec![]);       // empty OR
        let root = builder.and(vec![a, inner1, empty_or, inner2]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let root_id = expr.root_id().unwrap();
        let input = expr.to_syntax_string(&interner);
        normalize(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "(or)");
    }

    /// Realistic test: OR root with an empty AND child
    ///
    /// Input: (or (or (A)) (and) (B) (or))
    /// Expected: (and) <- empty AND absorbs the OR root
    #[test]
    fn test_realistic_or_flatten() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let inner_or1 = builder.or(vec![a]);
        let empty_and = builder.and(vec![]);
        let empty_or2 = builder.or(vec![]);
        let root = builder.or(vec![inner_or1, empty_and, b, empty_or2]);

        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let root_id = expr.root_id().unwrap();
        let input = expr.to_syntax_string(&interner);
        normalize(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert_eq!(output, "(and)");
    }
}


#[cfg(test)]
mod flatten_and_or_node_tests {
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::syntax::SyntaxDisplay;
    use super::*;

    /// Test flattening a root AND node with nested AND children.
    ///
    /// Input: (and (A) (and (B) (C)) (D))
    /// Expected: (and (A) (B) (C) (D))
    #[test]
    fn test_flatten_root_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let d = builder.atomic_formula("D", vec![]);
        let inner_and = builder.and(vec![b, c]);
        let root = builder.and(vec![a, inner_and, d]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let root_id = expr.root_id().unwrap();
        let input = expr.to_syntax_string(&interner);
        flatten_and_or_node(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 4); // A, B, C, D
        assert_eq!(output, "(and (A) (B) (C) (D))");
    }

    /// Test flattening a root OR node with nested OR children.
    ///
    /// Input: (or (A) (or (B) (C)))
    /// Expected: (or (A) (B) (C))
    #[test]
    fn test_flatten_root_or() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let inner_or = builder.or(vec![b, c]);
        let root = builder.or(vec![a, inner_or]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let root_id = expr.root_id().unwrap();
        let input = expr.to_syntax_string(&interner);
        flatten_and_or_node(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 3); // A, B, C
        assert_eq!(output, "(or (A) (B) (C))");
    }

    /// Test flattening nested AND children in a non-trivial root.
    ///
    /// Input: (and (A) (and (B) (C)) (D))
    /// Expected: (and (A) (B) (C) (D))
    #[test]
    fn test_flatten_non_root_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let c = builder.atomic_formula("C", vec![]);
        let d = builder.atomic_formula("D", vec![]);
        let inner_and = builder.and(vec![b, c]);
        let root = builder.and(vec![a, inner_and, d]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        flatten_and_or_node(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 4); // A, B, C, D
        assert_eq!(output, "(and (A) (B) (C) (D))");
    }

    /// Test that a node with no nested AND/OR is unchanged.
    ///
    /// Input: (and (A) (B))
    /// Expected: (and (A) (B))
    #[test]
    fn test_no_nested_nodes() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let root = builder.and(vec![a, b]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let root_id = expr.root_id().unwrap();
        let input = expr.to_syntax_string(&interner);
        flatten_and_or_node(root_id, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(root_id).unwrap();
        assert_eq!(root_node.children().len(), 2);
        assert_eq!(output, "(and (A) (B))");
    }
}

#[cfg(test)]
mod deduplicate_and_or_node_tests {
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::syntax::SyntaxDisplay;
    use super::*;

    /// Test NodeId-based deduplication in a root AND node.
    ///
    /// Input: (and (A) (A) (B))
    /// Expected: (and (A) (B))
    #[test]
    fn test_root_and_nodeid_duplicates() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let root = builder.and(vec![a, a, b]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);
        assert_eq!(output, "(and (A) (B))");
    }

    /// Test NodeId-based deduplication in a root OR node.
    ///
    /// Input: (or (A) (B) (B))
    /// Expected: (or (A) (B))
    #[test]
    fn test_root_or_nodeid_duplicates() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let root = builder.or(vec![a, b, b]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 2);
        assert_eq!(output, "(or (A) (B))");
    }

    /// Test NodeId-based deduplication preserves non-duplicated children.
    ///
    /// Input: (and (A) (B))
    /// Expected: (and (A) (B))
    #[test]
    fn test_and_no_duplicates() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let root = builder.and(vec![a, b]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.children().len(), 2);
        assert_eq!(output, "(and (A) (B))");
    }

    /// Test structural deduplication in a root AND node with duplicate subtrees.
    ///
    /// Input: (and (and (A) (B)) (and (A) (B)) (C))
    /// Expected: (and (and (A) (B)) (C))
    #[test]
    fn test_root_and_structural_duplicates() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let inner1 = builder.and(vec![a, b]);
        let inner2 = builder.and(vec![a, b]);
        let c = builder.atomic_formula("C", vec![]);
        let root = builder.and(vec![inner1, inner2, c]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);
        assert_eq!(output, "(and (and (A) (B)) (C))");
    }

    /// Test structural deduplication in a root OR node with duplicate subtrees.
    ///
    /// Input: (or (or (A) (B)) (or (A) (B)) (C))
    /// Expected: (or (or (A) (B)) (C))
    #[test]
    fn test_root_or_structural_duplicates() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let inner1 = builder.or(vec![a, b]);
        let inner2 = builder.or(vec![a, b]);
        let c = builder.atomic_formula("C", vec![]);
        let root = builder.or(vec![inner1, inner2, c]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 2);
        assert_eq!(output, "(or (or (A) (B)) (C))");
    }

}

#[cfg(test)]
mod simplify_tautologies_and_contradictions_tests {
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::syntax::SyntaxDisplay;
    use super::*;

    /// Input: (or A (not A))
    /// Expected output: (and)  // tautology in OR -> true
    #[test]
    fn test_or_tautology() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let not_a = builder.not(a);
        let root = builder.or(vec![a, not_a]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        let changed = simplify_tautologies_and_contradictions(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert!(changed);
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And); // true
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(and)");
    }

    /// Input: (and A (not A))
    /// Expected output: (or)  // contradiction in AND -> false
    #[test]
    fn test_and_contradiction() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let not_a = builder.not(a);
        let root = builder.and(vec![a, not_a]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        let changed = simplify_tautologies_and_contradictions(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert!(changed);
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or); // false
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(or)");
    }

    /// Input: (or A B)  // no tautology
    /// Expected output: unchanged
    #[test]
    fn test_or_no_tautology() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let root = builder.or(vec![a, b]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        let changed = simplify_tautologies_and_contradictions(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert!(!changed);
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(output, "(or (A) (B))");
    }

    /// Input: (and A B)  // no contradiction
    /// Expected output: unchanged
    #[test]
    fn test_and_no_contradiction() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let root = builder.and(vec![a, b]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        let changed = simplify_tautologies_and_contradictions(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);

        print!("{} -> {} ", input, output);
        assert!(!changed);
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(output, "(and (A) (B))");
    }
}

#[cfg(test)]
mod reduce_single_and_or_node_tests {
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::syntax::SyntaxDisplay;
    use super::*;
    /// Test root AND with a single child
    ///
    /// Input: (and A)
    /// Expected: A
    #[test]
    fn test_root_and_single_child() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let c = builder.predicate("A");
        let root = builder.and(vec![c]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        reduce_single_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Predicate);
    }

    /// Test non-root AND with a single child
    ///
    /// Input: (and (and A))
    /// Expected: (and A)
    #[test]
    fn test_and_single_child_non_root() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.predicate("A");
        let inner_and = builder.and(vec![a]);
        let root = builder.and(vec![inner_and]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        reduce_single_and_or_node(inner_and, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        // Root should still be AND
        assert_eq!(root_node.kind(), ExprKind::And);
        // Its only child should be the predicate
        assert_eq!(root_node.children().len(), 1);
        let child_node = expr.try_node(root_node.children()[0]).unwrap();
        assert_eq!(child_node.kind(), ExprKind::Predicate);
    }

    /// Test root OR with a single child
    ///
    /// Input: (or A)
    /// Expected: A
    #[test]
    fn test_root_or_single_child() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let d = builder.predicate("A");
        let root = builder.or(vec![d]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        reduce_single_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Predicate);
    }

    /// Test non-root OR with a single child
    ///
    /// Input: (or (or A))
    /// Expected: (or A)
    #[test]
    fn test_or_single_child_non_root() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let b = builder.predicate("A");
        let inner_or = builder.or(vec![b]);
        let root = builder.or(vec![inner_or]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        reduce_single_and_or_node(inner_or, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        // Root should still be OR
        assert_eq!(root_node.kind(), ExprKind::Or);
        // Its only child should be the predicate
        assert_eq!(root_node.children().len(), 1);
        let child_node = expr.try_node(root_node.children()[0]).unwrap();
        assert_eq!(child_node.kind(), ExprKind::Predicate);
    }

    /// Test that nodes with multiple children are unchanged
    ///
    /// Input: (and A B)
    /// Expected: (and A B)
    #[test]
    fn test_and_multiple_children() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.predicate("A");
        let b = builder.predicate("B");
        let root = builder.and(vec![a, b]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        reduce_single_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);
    }
}
#[cfg(test)]
mod simplify_empty_and_or_node_tests {
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::syntax::SyntaxDisplay;
    use super::*;

    /// Test that an AND node with an empty AND child removes the empty child.
    ///
    /// Input: (and (and))
    /// Expected: (and)
    #[test]
    fn test_and_with_empty_and_child() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_and = builder.and(vec![]);
        let root = builder.and(vec![empty_and]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_empty_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(and)");
    }

    /// Test absorbing child: AND node with an empty OR child.
    ///
    /// Input: (and (or))
    /// Expected: (or)
    #[test]
    fn test_and_absorbing_or() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_or = builder.or(vec![]);
        let root = builder.and(vec![empty_or]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_empty_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(or)");
    }

    /// Test that non-empty children of AND are preserved.
    ///
    /// Input: (and (A) (B))
    /// Expected: (and (A) (B))
    #[test]
    fn test_and_keep_children() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let root = builder.and(vec![a, b]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_empty_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);
        assert!(root_node.children().iter().all(|&id| {
            expr.try_node(id).unwrap().kind() == ExprKind::AtomicFormula
        }));
        assert_eq!(output, "(and (A) (B))");
    }

    /// Test mixed children in AND: empty AND, empty OR, and an atomic formula.
    ///
    /// Input: (and (and) (or) (A))
    /// Expected: (or)
    #[test]
    fn test_and_mixed_children_absorb() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_and = builder.and(vec![]);
        let empty_or = builder.or(vec![]);
        let a = builder.atomic_formula("A", vec![]);

        let root = builder.and(vec![empty_and, empty_or, a]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_empty_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(or)");
    }

    /// Test that an OR node with an empty OR child removes the empty child.
    ///
    /// Input: (or (or))
    /// Expected: (or)
    #[test]
    fn test_or_with_empty_or_child() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_or = builder.or(vec![]);
        let root = builder.or(vec![empty_or]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_empty_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(or)");
    }

    /// Test absorbing child: OR node with an empty AND child.
    ///
    /// Input: (or (and))
    /// Expected: (and)
    #[test]
    fn test_or_absorbing_and() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_and = builder.and(vec![]);
        let root = builder.or(vec![empty_and]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_empty_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(and)");
    }

    /// Test that non-empty children of OR are preserved.
    ///
    /// Input: (or (A) (B))
    /// Expected: (or (A) (B))
    #[test]
    fn test_or_keep_children() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let a = builder.atomic_formula("A", vec![]);
        let b = builder.atomic_formula("B", vec![]);
        let root = builder.or(vec![a, b]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_empty_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::Or);
        assert_eq!(root_node.children().len(), 2);
        assert!(root_node.children().iter().all(|&id| {
            expr.try_node(id).unwrap().kind() == ExprKind::AtomicFormula
        }));
        assert_eq!(output, "(or (A) (B))");
    }

    /// Test mixed children in OR: empty OR, empty AND, and an atomic formula.
    ///
    /// Input: (or (or) (and) (A))
    /// Expected: (and)
    #[test]
    fn test_or_mixed_children_absorb() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let empty_or = builder.or(vec![]);
        let empty_and = builder.and(vec![]);
        let a = builder.atomic_formula("A", vec![]);

        let root = builder.or(vec![empty_or, empty_and, a]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        simplify_empty_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert!(root_node.children().is_empty());
        assert_eq!(output, "(and)");
    }

    #[test]
    /// Test fusion of multiple WHENs with the same effect under an AND node.
    /// Input: (and (when C1 E) (when C2 E))
    /// Expected: (when (or C1 C2) E)
    #[test]
    fn test_when_merge_same_effect() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let c1 = builder.atomic_formula("C1", vec![]);
        let c2 = builder.atomic_formula("C2", vec![]);
        let e  = builder.atomic_formula("E", vec![]);

        let w1 = builder.when(c1, e);
        let w2 = builder.when(c2, e);

        let root = builder.and(vec![w1, w2]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        // Check that the root is a WHEN node after merge
        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::When);
        assert_eq!(root_node.children().len(), 2);

        // First child of WHEN should be an OR node combining conditions
        let cond_node = expr.try_node(root_node.children()[0]).unwrap();
        assert_eq!(cond_node.kind(), ExprKind::Or);
        assert_eq!(cond_node.children().len(), 2);
        let c1 = expr.try_node(cond_node.children()[0]).unwrap();
        assert_eq!(c1.kind(), ExprKind::AtomicFormula);
        let c2 = expr.try_node(cond_node.children()[1]).unwrap();
        assert_eq!(c2.kind(), ExprKind::AtomicFormula);

        // Second child of WHEN should be the effect node
        let eff_node = expr.try_node(root_node.children()[1]).unwrap();
        assert_eq!(eff_node.kind(), ExprKind::AtomicFormula);
    }


    /// Test that WHENs with different effects are not merged.
    /// Input: (and (when C1 E1) (when C2 E2))
    /// Expected: (and (when C1 E1) (when C2 E2))
    #[test]
    fn test_when_not_merge_different_effect() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let c1 = builder.atomic_formula("C1", vec![]);
        let c2 = builder.atomic_formula("C2", vec![]);
        let e1 = builder.atomic_formula("E1", vec![]);
        let e2 = builder.atomic_formula("E2", vec![]);

        let w1 = builder.when(c1, e1);
        let w2 = builder.when(c2, e2);

        let root = builder.and(vec![w1, w2]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::And);
        assert_eq!(root_node.children().len(), 2);
        assert!(root_node.children().iter().all(|&id| {
            expr.try_node(id).unwrap().kind() == ExprKind::When
        }));
    }

    /// Test WHEN with a single condition is preserved as-is (no OR created).
    /// Input: (and (when C E))
    /// Expected: (when C E)
    #[test]
    fn test_when_single_condition() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);

        let c = builder.atomic_formula("C", vec![]);
        let e = builder.atomic_formula("E", vec![]);

        let w = builder.when(c, e);
        let root = builder.and(vec![w]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::When);
        assert_eq!(root_node.children().len(), 2);
        let cond_node = expr.try_node(root_node.children()[0]).unwrap();
        assert_eq!(cond_node.kind(), ExprKind::AtomicFormula);
        let eff_node = expr.try_node(root_node.children()[1]).unwrap();
        assert_eq!(eff_node.kind(), ExprKind::AtomicFormula);
    }

    /// Test fusion of identical WHEN conditions under an AND node.
    /// Input: (and (when C E) (when C E))
    /// Expected: (when C E) after OR simplification
    #[test]
    fn test_when_merge_with_or_simplification() {
        let mut interner = StringInterner::new();
        let mut builder = ExprBuilder::new(&mut interner);
        let c = builder.atomic_formula("C", vec![]);
        let e = builder.atomic_formula("E", vec![]);
        let w1 = builder.when(c, e);
        let w2 = builder.when(c, e);
        let root = builder.and(vec![w1, w2]);
        builder.set_root(root).unwrap();
        let mut expr = builder.finish();

        let input = expr.to_syntax_string(&interner);
        normalize(root, &mut expr).unwrap();
        let output = expr.to_syntax_string(&interner);
        print!("{} -> {} ", input, output);

        let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
        assert_eq!(root_node.kind(), ExprKind::When);
        assert_eq!(root_node.children().len(), 2);
        let cond_node = expr.try_node(root_node.children()[0]).unwrap();
        assert_eq!(cond_node.kind(), ExprKind::AtomicFormula);
        let eff_node = expr.try_node(root_node.children()[1]).unwrap();
        assert_eq!(eff_node.kind(), ExprKind::AtomicFormula);

    }

}
