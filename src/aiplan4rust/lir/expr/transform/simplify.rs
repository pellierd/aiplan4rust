use crate::aiplan4rust::lir::expr::{Expr, ExprError, ExprKind, ExprNode};
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
pub fn simplify(expr: &mut Expr) -> Result<(), ExprError> {
    let Some(root_id) = expr.root_id() else { return Ok(()); };

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
        simplify_node(node_id, expr)?;
    }

    /// TO ADD
    /// Factorise les parties communes des expressions.
    /// Exemple: `(A ∧ B) ∨ (A ∧ C) -> A ∧ (B ∨ C)`.
    Ok(())
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
fn simplify_node(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    let kind = expr.try_node(node_id)?.kind();

    match kind {
        ExprKind::And | ExprKind::Or => {
            simplify_and_or_node(node_id, expr)?;
        }
        ExprKind::Not => {
            simplify_not_node(node_id, expr)?;
        }
        _ => {} // Other node kinds are skipped
    }

    Ok(())
}

/// Simplifies an AND or OR node in a PDDL expression tree.
///
/// This function performs several simplifications on a node of type `AND` or `OR`,
/// processing the node in place. It does nothing if the node is of another kind.
///
/// The simplifications are performed in the following order:
/// 1. **Flatten nested nodes**: If the node has children of the same kind, they are
///    lifted up to the current node. For example:
///    `(and A (and B C))` becomes `(and A B C)`.
/// 2. **Structural deduplication**: Duplicate subtrees are removed using a
///    structural hash (`sub_expr_hash`). For example:
///    `(and (and A B) (and A B))` becomes `(and (and A B))`.
/// 3. **Single-child reduction**: If the node has only one child after deduplication,
///    it is replaced by that child. For example:
///    `(and A)` becomes `A`.
/// 4. **Empty-node simplification**: If the node has no children, it is replaced
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
/// - It only operates on `AND` or `OR` nodes; other kinds of nodes are skipped silently.
///
/// # Example
/// ```ignore
/// // Suppose expr represents: (and A (and B C) (and A B))
/// let node_id = expr.root_id().unwrap();
/// simplify_and_or_node(node_id, &mut expr)?;
/// // After simplification, the expression becomes: (and A B C)
/// ```
fn simplify_and_or_node(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    // Borrow the node immutably to check its kind.
    // We only want to simplify AND or OR nodes.
    let kind = expr.try_node(node_id)?.kind();

    // If the node is neither AND nor OR, skip simplification.
    if kind != ExprKind::And && kind != ExprKind::Or {
        return Ok(());
    }

    // Step 1: Flatten nested AND/OR nodes of the same kind.
    // For example, (and A (and B C)) -> (and A B C)
    flatten_and_or_node(node_id, expr)?;

    // Step 2: Deduplicate children structurally.
    // Removes duplicate subtrees based on `sub_expr_hash`.
    // For example, (and (and A B) (and A B)) -> (and (and A B))
    deduplicate_and_or_node(node_id, expr, true)?;

    // Step 3: Reduce AND/OR nodes that have a single child.
    // For example, (and A) -> A
    reduce_single_and_or_node(node_id, expr)?;

    // Step 4: Simplify empty AND/OR nodes.
    // For example, (and) -> true or (or) -> false depending on your semantics.
    simplify_empty_and_or_node(node_id, expr)?;

    // Indicate that simplification for this node succeeded.
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
/// - If `node_id` is not an AND or OR node, the function does nothing and returns `Ok(())`.
///
/// # Returns
/// - `Ok(())` if the operation succeeds.
/// - `Err(ExprError)` if accessing any node fails.
///
/// # Notes
/// - Uses `std::mem::take` to temporarily take ownership of children vectors, avoiding
///   borrow checker conflicts.
/// - Useful for simplifying logical expressions in PDDL-like ASTs by reducing unnecessary nesting.
fn flatten_and_or_node(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    // Borrow the node immutably to check its kind.
    let kind = expr.try_node(node_id)?.kind();

    // Only AND or OR nodes are flattened; skip other node types.
    if kind != ExprKind::And && kind != ExprKind::Or {
        return Ok(());
    }

    // Take ownership of the current children vector to avoid borrow conflicts.
    // `children_mut()` gives a mutable reference; `std::mem::take` replaces it with an empty vec.
    let children = std::mem::take(expr.try_node_mut(node_id)?.children_mut());

    // Prepare a new vector to hold the flattened children.
    let mut flat = Vec::new();

    // Iterate over each child of the node.
    for child_id in children {
        // Borrow the child immutably to check its kind.
        let child_kind = expr.try_node(child_id)?.kind();

        // If the child is of the same kind as the parent (nested AND/OR),
        // take ownership of the child's children and append them to `flat`.
        if child_kind == kind {
            flat.extend(std::mem::take(expr.try_node_mut(child_id)?.children_mut()));
        } else {
            // Otherwise, keep the child as-is.
            flat.push(child_id);
        }
    }

    // After flattening all children, assign the new flattened vector back to the node.
    expr.try_node_mut(node_id)?.set_children(flat);

    Ok(())
}

/// Removes duplicate children from an AND/OR node, optionally using structural comparison.
///
/// # Parameters
/// - `node_id`: the ID of the node to deduplicate. Only AND/OR nodes are affected.
/// - `expr`: mutable reference to the expression tree.
/// - `structural`: if true, deduplicate children based on subtree structure (`sub_expr_hash`),
///   otherwise deduplicate based on `NodeId`.
///
/// # Behavior
/// - If the node is an `AND` or `OR`:
///     - `structural = false`: all duplicate children with the same `NodeId` are removed,
///       keeping only the first occurrence of each child.
///     - `structural = true`: all duplicate children whose subtrees are identical (same structural hash)
///       are removed, keeping only the first occurrence of each unique subtree.
/// - The original order of the children is preserved.
/// - Non-AND/OR nodes are silently skipped (no error is returned).
///
/// # Returns
/// - `Ok(())` if the operation succeeds.
/// - `Err(ExprError)` if accessing a node or computing a subtree hash fails.
///
/// # Notes
/// - Uses an immutable borrow to read children first, then a mutable borrow to write them back,
///   avoiding Rust borrow conflicts.
/// - Structural deduplication is more expensive but allows removing logically equivalent subtrees.
///
/// # Example
/// ```ignore
/// // AND node with duplicate NodeIds: (and A A B)
/// // After deduplication with structural = false: (and A B)
///
/// // AND node with duplicate subtrees: (and (and A B) (and A B))
/// // After deduplication with structural = true: (and (and A B))
/// ```
fn deduplicate_and_or_node(
    node_id: NodeId,
    expr: &mut Expr,
    structural: bool,
) -> Result<(), ExprError> {
    // Immutably borrow the node to read kind and children
    let node = expr.try_node(node_id)?;

    // Only process AND or OR nodes; skip others silently
    match node.kind() {
        ExprKind::And | ExprKind::Or => {}
        _ => return Ok(()),
    }

    // Prepare a vector to store deduplicated children
    let mut deduped = Vec::with_capacity(node.children().len());

    if structural {
        // Structural deduplication: track hashes of seen subtrees
        let mut seen_hashes = std::collections::HashSet::new();

        for &child_id in node.children() {
            // Compute the structural hash of each child subtree
            let h = expr.sub_expr_hash(child_id)?;

            // If this hash hasn't been seen yet, keep the child
            if seen_hashes.insert(h) {
                deduped.push(child_id);
            }
        }
    } else {
        // NodeId deduplication: track NodeIds already seen
        let mut seen_ids = std::collections::HashSet::new();

        for &child_id in node.children() {
            // If this NodeId hasn't been seen yet, keep it
            if seen_ids.insert(child_id) {
                deduped.push(child_id);
            }
        }
    }

    // Mutably borrow the node to write back the deduplicated children
    let node = expr.try_node_mut(node_id)?;
    node.set_children(deduped);

    // Return success
    Ok(())
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
/// - `Ok(())` on success (even if no reduction was performed).
/// - `Err(ExprError)` if accessing a node or the root fails.
///
/// # Note
/// - This function does not return an error for non-AND/OR nodes; it silently skips them.
/// - Intended to be called as part of the simplification pipeline on AND/OR nodes only.
fn reduce_single_and_or_node(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), ExprError> {
    // Borrow the node immutably to check its kind
    let node = expr.try_node(node_id)?;

    // Only AND/OR nodes are considered
    match node.kind() {
        ExprKind::And | ExprKind::Or => {}
        _ => return Ok(()), // silently skip non-AND/OR
    }

    let children = node.children();
    // Only reduce if there is exactly one child
    if children.len() != 1 {
        return Ok(());
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

    Ok(())
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
/// - `Ok(())` if the simplification completes successfully.
/// - `Err(ExprError)` if accessing a child node fails.
///
/// # Notes
/// - Copies only the `NodeId`s of children (lightweight integers).
/// - Temporarily takes ownership of the children vector via `std::mem::take` for safe mutable operations.
/// - Preserves PDDL semantics:
///   - `(and)` with no children → `true`
///   - `(or)` with no children → `false`
///   - No explicit `true` or `false` constants are introduced.
///
/// # Examples
/// ```ignore
/// // Example 1: AND node with empty AND child
/// // Input: (and (and))
/// // Output after simplification: (and)
/// // Semantic meaning: true
///
/// // Example 2: AND node with empty OR child
/// // Input: (and (or))
/// // Output after simplification: (or)
/// // Semantic meaning: false
/// ```
fn simplify_empty_and_or_node(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), ExprError> {
    // 1. Access the node corresponding to node_id (immutable borrow)
    let node = expr.try_node(node_id)?;
    let node_kind = node.kind(); // get the node type: And / Or / other

    // 2. Ensure this node is AND or OR
    //    If not, nothing to do
    if node_kind != ExprKind::And && node_kind != ExprKind::Or {
        return Ok(());
    }

    // 3. Prepare a new vector to store the children that will remain
    //    Only copies NodeIds (integers), which is lightweight
    let mut new_children = Vec::with_capacity(node.children().len());

    // 4. Iterate over all children of the node
    for &child_id in node.children() {
        let child = expr.try_node(child_id)?; // immutable borrow of the child
        let child_kind = child.kind();        // child's type
        let child_empty = child.children().is_empty(); // check if child is empty

        // -------- Case 1: absorbing child --------
        // If the child is empty and of the opposite type:
        //   - (and (or)) → becomes (or) → false
        //   - (or (and)) → becomes (and) → true
        if (node_kind == ExprKind::And && is_empty_or(&child))
            || (node_kind == ExprKind::Or && is_empty_and(&child))
        {
            // Mutably borrow the parent node only here
            let mut node_mut = expr.try_node_mut(node_id)?;
            node_mut.set_kind(child_kind);  // replace the node type
            node_mut.set_children(vec![]);  // clear all children
            return Ok(());                  // simplification done
        }

        // -------- Case 2: neutral child --------
        // If the child is empty and of the same type:
        //   - (and (and)) → ignore the child → remains (and) → true
        //   - (or (or))   → ignore the child → remains (or)  → false
        if child_kind == node_kind && child_empty {
            continue; // skip this child
        }

        // -------- Case 3: keep child --------
        // All other children are preserved
        new_children.push(child_id);
    }

    // -------- Case 3: update node children at the end --------
    // Mutably borrow the parent node once
    let mut node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_children(new_children);

    Ok(())
}

/// Checks whether a node is an empty AND `(and)` node.
///
/// # Parameters
/// - `node`: A reference to the `ExprNode` to check.
///
/// # Returns
/// - `true` if the node is of kind `ExprKind::And` and has no children (i.e., `(and)` in PDDL
///     semantics, considered `true`).
/// - `false` otherwise.
fn is_empty_and(node: &ExprNode) -> bool {
    node.kind() == ExprKind::And && node.children().is_empty()
}

/// Checks whether a node is an empty OR `(or)` node.
///
/// # Parameters
/// - `node`: A reference to the `ExprNode` to check.
///
/// # Returns
/// - `true` if the node is of kind `ExprKind::Or` and has no children (i.e., `(or)` in PDDL
///     semantics, considered `false`).
/// - `false` otherwise.
fn is_empty_or(node: &ExprNode) -> bool {
    node.kind() == ExprKind::Or && node.children().is_empty()
}

/// Simplifies a double negation in a PDDL expression tree.
///
/// This function transforms a node of type `Not` that has exactly one child,
/// where the child is also a `Not`, into the grandchild node, effectively
/// eliminating the double negation. The transformation is done in place
/// without cloning the subtree, using `std::mem::take()` to move content and children.
///
/// # Parameters
/// - `node_id`: The `NodeId` of the node to simplify. Must be a `Not` node.
/// - `expr`: Mutable reference to the expression tree containing the node.
///
/// # Behavior
/// - If the node is not a `Not`, the function returns `Ok(())` and does nothing.
/// - If the node has zero or multiple children, the function returns `Ok(())`.
/// - If the node has exactly one child, and that child is a `Not` with exactly
///   one child, the double negation is removed and the current node is replaced
///   by the grandchild node (type, content, and children are moved).
/// - Otherwise, the function returns `Ok(())` without modifying the tree.
///
/// # Returns
/// - `Ok(())` if the simplification completes successfully or if no simplification
///   is applicable.
/// - `Err(ExprError)` if accessing nodes or mutating the tree fails.
///
/// # Notes
/// - The function assumes the AST is generally well-formed (each `Not` should have
///   exactly one child). A `debug_assert!` is used to check this invariant during
///   debug builds.
/// - The simplification does not clone subtrees; it moves the content and children
///   of the grandchild node.
/// - This function is intended to be called as part of a post-order traversal
///   of the expression tree.
fn simplify_not_node(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprError> {
    let node = expr.try_node(node_id)?;
    if node.kind() != ExprKind::Not {
        return Ok(()); // Not a NOT node, ignore
    }

    debug_assert!(node.children().len() == 1, "Not node must have exactly one child");
    let children = node.children();
    if children.len() != 1 {
        return Ok(()); // safety guard in case of malformed AST
    }

    let child_id = children[0];
    let child = expr.try_node(child_id)?;
    debug_assert!(child.children().len() == 1 || child.kind() != ExprKind::Not,
                  "If child is a NOT, it must have exactly one child");

    if child.kind() != ExprKind::Not || child.children().len() != 1 {
        return Ok(()); // only simplify double negation
    }

    let grandchild_id = child.children()[0];

    // Take ownership of grandchild node completely
    let grandchild_node = {
        let grandchild_mut = expr.try_node_mut(grandchild_id)?;
        let kind = grandchild_mut.kind();
        let content = std::mem::take(grandchild_mut.content_mut());
        let children = std::mem::take(grandchild_mut.children_mut());
        (kind, content, children)
    };

    // Mutate current NOT node in place
    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.set_kind(grandchild_node.0);
    *node_mut.content_mut() = grandchild_node.1; // move content
    node_mut.set_children(grandchild_node.2);    // move children

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::syntax::SyntaxDisplay;

    #[cfg(test)]
    mod simplify_and_or_node_simplify_tests {
        use super::*;
        /// Test flattening and structural deduplication on a complex nested AND node.
        ///
        /// Input: (and (and A B) (and B C) (and (and A B) D))
        /// Expected after simplify: (and A B C D)
        #[test]
        fn test_complex_nested_and_deduplication() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);
            let a = builder.predicate("A");
            let b = builder.predicate("B");
            let c = builder.predicate("C");
            let d = builder.predicate("D");
            let inner1 = builder.and(vec![a, b]);
            let inner2 = builder.and(vec![b, c]);
            let inner3 = builder.and(vec![inner1, d]);
            let root = builder.and(vec![inner1, inner2, inner3]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();
            let input = expr.to_syntax_string(&interner);
            simplify(&mut expr).unwrap();
            let output = expr.to_syntax_string(&interner);
            print!("{} -> {} ", input, output);
            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::And);
            let root_children: Vec<_> = root_node.children().iter().map(|&id| expr.try_node(id).unwrap().kind()).collect();
            assert!(root_children.iter().all(|&k| k == ExprKind::Predicate));
            assert_eq!(root_node.children().len(), 4);
        }
        /// Test simplification of a root AND with nested AND children and duplicates.
        ///
        /// Input: (and A (and B C) (and B C))
        /// Expected after simplify: (and A B C)
        #[test]
        fn test_root_and_structural_simplification() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
            let c = builder.predicate("C");

            let inner1 = builder.and(vec![b, c]);
            let inner2 = builder.and(vec![b, c]);

            let root = builder.and(vec![a, inner1, inner2]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            simplify(&mut expr).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::And);
            assert_eq!(root_node.children().len(), 3); // A, B, C
        }

        /// Test simplification of a root OR with nested OR children and duplicates.
        ///
        /// Input: (or A (or B C) (or B C))
        /// Expected after simplify: (or A B C)
        #[test]
        fn test_root_or_structural_simplification() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
            let c = builder.predicate("C");

            let inner1 = builder.or(vec![b, c]);
            let inner2 = builder.or(vec![b, c]);

            let root = builder.or(vec![a, inner1, inner2]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            simplify(&mut expr).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::Or);
            assert_eq!(root_node.children().len(), 3); // A, B, C
        }

        /// Test simplification of an AND node with a single child after flattening/deduplication.
        ///
        /// Input: (and (and A))
        /// Expected after simplify: A
        #[test]
        fn test_and_single_child_reduction() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let inner = builder.and(vec![a]);
            let root = builder.and(vec![inner]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            simplify(&mut expr).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            // After simplification, root should be replaced by A
            assert_eq!(root_node.kind(), ExprKind::Predicate);
        }

        /// Test simplification of an empty AND node.
        ///
        /// Input: (and)
        /// Expected: neutral value (depends on semantics)
        #[test]
        fn test_empty_and_node() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let root = builder.and(vec![]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            simplify(&mut expr).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            // Depending on semantics, could still be AND
            assert!(root_node.kind() == ExprKind::And);
        }

        /// Test simplification of an empty OR node.
        ///
        /// Input: (or)
        /// Expected: neutral value (depends on semantics)
        #[test]
        fn test_empty_or_node() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let root = builder.or(vec![]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            simplify(&mut expr).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            // Depending on semantics, could still be OR
            assert!(root_node.kind() == ExprKind::Or);
        }
    }

    // --- Tests for flatten_and_or_node ---
    mod flatten_and_or_node_tests {
        use super::*;
        /// Test flattening a root AND node with nested AND children.
        ///
        /// Input: (and A (and B C) D)
        /// Expected: (and A B C D)
        #[test]
        fn test_flatten_root_and() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
            let c = builder.predicate("C");
            let d = builder.predicate("D");
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
        }

        /// Test flattening a root OR node with nested OR children.
        ///
        /// Input: (or A (or B C))
        /// Expected: (or A B C)
        #[test]
        fn test_flatten_root_or() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let x = builder.predicate("A");
            let y = builder.predicate("B");
            let z = builder.predicate("C");
            let inner_or = builder.or(vec![y, z]);
            let root = builder.or(vec![x, inner_or]);
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
        }

        /// Test flattening a nested AND/OR child (non-root node).
        ///
        /// Input: (and A (and B C) D)
        /// Expected: (and A B C D)
        #[test]
        fn test_flatten_non_root_and() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
            let c = builder.predicate("C");
            let d = builder.predicate("D");
            let inner_and = builder.and(vec![b, c]);
            let root = builder.and(vec![a, inner_and, d]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();
            let input = expr.to_syntax_string(&interner);
            flatten_and_or_node(inner_and, &mut expr).unwrap();
            let output = expr.to_syntax_string(&interner);
            print!("{} -> {} ", input, output);
            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::And);
            assert_eq!(root_node.children().len(), 3); // root children remain A, inner_and flattened, D
        }


        /// Test that a node with no nested AND/OR is unchanged.
        ///
        /// Input: (and A B)
        /// Expected: (and A B)
        #[test]
        fn test_no_nested_nodes() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
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
        }

        /// Test that non-AND/OR nodes are skipped silently.
        ///
        /// Input: A
        /// Expected: A
        #[test]
        fn test_non_and_or_node_skipped() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);
            let a = builder.predicate("A");
            builder.set_root(a).unwrap();
            let mut expr = builder.finish();
            let input = expr.to_syntax_string(&interner);
            flatten_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
            let output = expr.to_syntax_string(&interner);
            print!("{} -> {} ", input, output);
            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::Predicate);
        }

    }


    // --- Tests for deduplicate_and_or_node_tests ---
    mod deduplicate_and_or_node_tests {
        use super::*;
        /// Test NodeId-based deduplication in a root AND node.
        ///
        /// Input: (and A A B)
        /// Expected: (and A B)
        #[test]
        fn test_root_and_nodeid_duplicates() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
            let root = builder.and(vec![a, a, b]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr, false).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::And);
            assert_eq!(root_node.children().len(), 2);
        }

        /// Test NodeId-based deduplication in a root OR node.
        ///
        /// Input: (or A B B)
        /// Expected: (or A B)
        #[test]
        fn test_root_or_nodeid_duplicates() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
            let root = builder.or(vec![a, b, b]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr, false).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::Or);
            assert_eq!(root_node.children().len(), 2);
        }

        /// Test NodeId-based deduplication preserves non-duplicated children.
        ///
        /// Input: (and A B)
        /// Expected: (and A B)
        #[test]
        fn test_and_no_duplicates() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
            let root = builder.and(vec![a, b]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr, false).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.children().len(), 2);
        }

        /// Test structural deduplication in a root AND node with duplicate subtrees.
        ///
        /// Input: (and (and A B) (and A B) C)
        /// Expected: (and (and A B) C)
        #[test]
        fn test_root_and_structural_duplicates() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
            let inner1 = builder.and(vec![a, b]);
            let inner2 = builder.and(vec![a, b]); // structurally identical but different NodeId
            let c = builder.predicate("C");
            let root = builder.and(vec![inner1, inner2, c]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr, true).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::And);
            assert_eq!(root_node.children().len(), 2);
        }

        /// Test structural deduplication in a root OR node with duplicate subtrees.
        ///
        /// Input: (or (or A B) (or A B) C)
        /// Expected: (or (or A B) C)
        #[test]
        fn test_root_or_structural_duplicates() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
            let inner1 = builder.or(vec![a, b]);
            let inner2 = builder.or(vec![a, b]);
            let c = builder.predicate("C");
            let root = builder.or(vec![inner1, inner2, c]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr, true).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::Or);
            assert_eq!(root_node.children().len(), 2);
        }

        /// Test structural deduplication in a root OR node with duplicate subtrees,
        /// verifying that duplicates are removed regardless of their order.
        ///
        /// Input: (or (or A B) (or A B) C)
        /// Expected: (or (or A B) C)
        #[test]
        fn test_root_or_structural_duplicates_order_independent() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            // Base predicates
            let a = builder.predicate("A");
            let b = builder.predicate("B");

            // Two structurally identical subtrees
            let inner1 = builder.or(vec![a, b]);
            let inner2 = builder.or(vec![b, a]); // reversed order

            let c = builder.predicate("C");

            // Root OR node containing duplicate subtrees
            let root = builder.or(vec![inner1, inner2, c]);
            builder.set_root(root).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            // Deduplicate structurally
            deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr, true).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::Or);

            // Check that only two children remain
            assert_eq!(root_node.children().len(), 2);

            // Optionally, verify that one of the OR subtrees is kept
            let kept_subtree = root_node.children()[0];
            let kept_node = expr.try_node(kept_subtree).unwrap();
            assert_eq!(kept_node.kind(), ExprKind::Or);

            // Ensure C is present as well
            let c_node = root_node.children().iter().find_map(|&id| {
                let node = expr.try_node(id).unwrap();
                if node.kind() == ExprKind::Predicate { Some(node) } else { None }
            });
            assert!(c_node.is_some());
        }

        /// Test that non-AND/OR nodes are skipped.
        ///
        /// Input: A
        /// Expected: A
        #[test]
        fn test_non_and_or_node_skipped() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            builder.set_root(a).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            deduplicate_and_or_node(expr.root_id().unwrap(), &mut expr, true).unwrap();
            let output = expr.to_syntax_string(&interner);

            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::Predicate);
        }
    }

    // --- Tests for reduce_single_and_or_node ---
    mod reduce_single_and_or_node_tests {
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

        /// Test that non-AND/OR nodes are skipped
        ///
        /// Input: A
        /// Expected: A
        #[test]
        fn test_non_and_or_node() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            builder.set_root(a).unwrap();
            let mut expr = builder.finish();

            let input = expr.to_syntax_string(&interner);
            reduce_single_and_or_node(expr.root_id().unwrap(), &mut expr).unwrap();
            let output = expr.to_syntax_string(&interner);
            print!("{} -> {} ", input, output);

            let root_node = expr.try_node(expr.root_id().unwrap()).unwrap();
            assert_eq!(root_node.kind(), ExprKind::Predicate);
        }
    }
    // --- Tests for simplify_empty_and_or_node ---
    mod simplify_empty_and_or_node_tests {
        use super::*;

        /// Test that an AND node with an empty AND child removes the empty child.
        ///
        /// Input: (and (and))
        /// Expected: (and) → true
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
        }

        /// Test absorbing child: AND node with an empty OR child.
        ///
        /// Input: (and (or))
        /// Expected: (or) → false
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
        }

        /// Test that non-empty children of AND are preserved.
        ///
        /// Input: (and A B)
        /// Expected: (and A B)
        #[test]
        fn test_and_keep_children() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
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
                expr.try_node(id).unwrap().kind() == ExprKind::Predicate
            }));
        }

        /// Test mixed children in AND: empty AND, empty OR, and a predicate (absorbing occurs).
        ///
        /// Input: (and (and) (or) A)
        /// Expected: (or) → false
        #[test]
        fn test_and_mixed_children_absorb() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let empty_and = builder.and(vec![]);
            let empty_or = builder.or(vec![]);
            let a = builder.predicate("A");

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
        }

        /// Test that an OR node with an empty OR child removes the empty child.
        ///
        /// Input: (or (or))
        /// Expected: (or) → false
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
        }

        /// Test absorbing child: OR node with an empty AND child.
        ///
        /// Input: (or (and))
        /// Expected: (and) → true
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
        }

        /// Test that non-empty children of OR are preserved.
        ///
        /// Input: (or A B)
        /// Expected: (or A B)
        #[test]
        fn test_or_keep_children() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let a = builder.predicate("A");
            let b = builder.predicate("B");
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
                expr.try_node(id).unwrap().kind() == ExprKind::Predicate
            }));
        }

        /// Test mixed children in OR: empty OR, empty AND, and a predicate (absorbing occurs).
        ///
        /// Input: (or (or) (and) A)
        /// Expected: (and) → true
        #[test]
        fn test_or_mixed_children_absorb() {
            let mut interner = StringInterner::new();
            let mut builder = ExprBuilder::new(&mut interner);

            let empty_or = builder.or(vec![]);
            let empty_and = builder.and(vec![]);
            let a = builder.predicate("A");

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
        }
    }
}
