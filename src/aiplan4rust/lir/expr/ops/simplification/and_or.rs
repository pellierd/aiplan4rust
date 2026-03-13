use std::collections::HashSet;
use crate::aiplan4rust::lir::expr::{Expr, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::expr::ops::error::ExprOpError;
use crate::aiplan4rust::tree::NodeId;

/// Simplifies an AND or OR node in a PDDL expression tree, including merging WHEN logic.
///
/// This function performs several simplifications on a node of typing `AND` or `OR`,
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
/// 4. **Merge WHEN logic**:
///    - All `When` logic among the children are grouped by their effect.
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
/// - `node_id`: The ID of the node to simplification.
/// - `logic`: Mutable reference to the expression tree containing the node.
///
/// # Returns
/// - `Ok(())` if the simplification succeeds.
/// - `Err(ExprError)` if accessing a node fails.
///
/// # Notes
/// - This function assumes that `logic` is a well-formed tree and that `node_id` exists.
/// - Only AND or OR nodes are simplified; other nodes are skipped silently.
/// - The `merge_when_in_place` step ensures that logically equivalent WHEN conditions
///   are grouped and avoids redundant branches in the expression.
/// - Sorting children in canonical order ensures that `deep_subexpr_eq` and other
///   structural equality checks behave consistently regardless of original child order.
///
/// # Example
/// ```ignore
/// // Suppose logic represents: (and A (and B C) (when X Y) (when Z Y))
/// let node_id = logic.root_id().unwrap();
/// normalize(node_id, &mut logic)?;
/// // After simplification, the expression becomes: (and A B C (when (or X Z) Y))
/// ```
pub fn simplify(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<(), ExprOpError> {

    // Step 1: Flatten nested AND/OR nodes of the same kind
    flatten_and_or_node(node_id, expr)?;

    // Step 2: Sort children in canonical order
    canonicalize_and_or_node(node_id, expr)?;

    // Step 3: Deduplicate structurally
    deduplicate_and_or_node(node_id, expr)?;

    // Step 4: Merge WHEN logic
    merge_when(node_id, expr)?; // merge WHEN logic grouped by effect

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
/// - `node_id`: The ID of the node to types. Only AND/OR nodes are affected.
/// - `logic`: Mutable reference to the expression tree containing the node.
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
/// - Useful for simplifying logical logic in PDDL-like ASTs by reducing unnecessary nesting.
fn flatten_and_or_node(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprOpError> {
    let kind = expr.try_node(node_id)?.kind();
    debug_assert!(kind == ExprKind::And || kind == ExprKind::Or);

    // 1. Vérification rapide : est-ce qu'un enfant est du même typing ?
    let needs_flattening = expr.try_node(node_id)?.children().iter().any(|&c| {
        expr.try_node(c).map(|n| n.kind() == kind).unwrap_or(false)
    });

    if !needs_flattening {
        return Ok(false);
    }

    // 2. Action : On reconstruit la liste
    let old_children = std::mem::take(expr.try_node_mut(node_id)?.children_mut());
    let mut flat = Vec::with_capacity(old_children.len());

    for child_id in old_children {
        if expr.try_node(child_id)?.kind() == kind {
            // On "aspire" les petits-enfants
            let mut grand_children = std::mem::take(expr.try_node_mut(child_id)?.children_mut());
            flat.append(&mut grand_children);
        } else {
            flat.push(child_id);
        }
    }

    expr.try_node_mut(node_id)?.set_children(flat);
    Ok(true)
}

/// Canonicalizes the children of an AND/OR node.
///
/// This function sorts the children of a commutative node (`AND` or `OR`) into a canonical order,
/// which ensures that structural equality checks and deduplication are independent of the original
/// order of the children.
///
/// # Parameters
/// - `node_id`: The ID of the node whose children should be canonicalized. Only AND/OR nodes are affected.
/// - `logic`: A mutable reference to the expression tree.
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
/// // After calling `canonicalize_and_or_children(node_id, &mut logic)`:
/// // Children are [1, 2, 3]
/// ```
fn canonicalize_and_or_node(node_id: NodeId, expr: &mut Expr) -> Result<(), ExprOpError> {
    // 1. Vérification rapide en lecture seule
    let node = expr.try_node(node_id)?;
    let kind = node.kind();

    if (kind != ExprKind::And && kind != ExprKind::Or) || node.children().len() <= 1 {
        return Ok(());
    }

    // 2. Tri in-place
    // On utilise sort_unstable car l'ordre relatif des NodeIds identiques n'importe pas,
    // et c'est généralement plus rapide car cela n'alloue pas de mémoire temporaire.
    let node_mut = expr.try_node_mut(node_id)?;
    node_mut.children_mut().sort_unstable();

    Ok(())
}

/// Removes duplicate children from an AND or OR node in an expression tree.
///
/// # Parameters
/// - `node_id`: The ID of the AND/OR node to deduplicate.
/// - `logic`: Mutable reference to the expression tree.
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
fn deduplicate_and_or_node(node_id: NodeId, expr: &mut Expr) -> Result<bool, ExprOpError> {
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

/// Checks for tautologies and contradictions in an AND/OR node using structural analysis
/// and static fact evaluation.
///
/// This pass performs two levels of simplification:
/// 1. **Semantic**: Using the `FactRegistry` to identify facts that are statically impossible or mandatory.
/// 2. **Structural**: Identifying pairs of $(\phi, \neg \phi)$ within the same node.
///
/// # Logic
/// - For **OR** nodes: `(or φ (not φ) ...)` is a tautology $\equiv$ `true`.
/// - For **AND** nodes: `(and φ (not φ) ...)` is a contradiction $\equiv$ `false`.
///
/// # Parameters
/// - `node_id`: [`NodeId`] of the AND/OR node to simplification.
/// - `logic`: Mutable reference to the expression arena.
/// - `fact_registry`: Optional reference to the [`InertiaRegistry`] for static analysis (Koehler/Inertia).
///
/// # Returns
/// - `Ok(true)` if the node was successfully reduced to a constant (true/false).
/// - `Ok(false)` if no simplification was performed.
/// - `Err(LogicError)` if an arena access or reduction operation fails.
///
/// # Steps
/// 1. **Early Exit**: Check if the node has children. Skip if empty.
/// 2. **Static Reduction**: For each child, consult the `FactRegistry`. If a child
///    short-circuits the parent (e.g., `false` found in `AND`), reduce immediately.
/// 3. **Structural Collection**: Track "positive" and "negated" children IDs.
/// 4. **Early Detection**: During collection, if the complement of the current child
///    is already in the set, a tautology/contradiction is found.
/// 5. **Finalize**: Update the node to a constant value using [`short_circuit_tautology`].
fn simplify_tautologies_and_contradictions(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprOpError> {
    let kind = expr.try_node(node_id)?.kind();
    let children = expr.try_node(node_id)?.children().to_vec();

    let mut positive_ids = HashSet::with_capacity(children.len());
    let mut negated_ids = HashSet::with_capacity(children.len());

    for child_id in children {
        let child_node = expr.try_node(child_id)?;
        let child_kind = child_node.kind();

        // --- 1. Constantes (Post-ordre result) ---
        // Si l'enfant est devenu True ou False via le registre au tour précédent
        match (kind, child_kind) {
            // Dans un AND, si un enfant est FALSE (Or vide), tout est FALSE
            (ExprKind::And, ExprKind::Or) if child_node.children().is_empty() => {
                expr.set_to_bool(node_id, false)?;
                return Ok(true);
            }
            // Dans un OR, si un enfant est TRUE (And vide), tout est TRUE
            (ExprKind::Or, ExprKind::And) if child_node.children().is_empty() => {
                expr.set_to_bool(node_id, true)?;
                return Ok(true);
            }
            _ => {}
        }

        // --- 2. Analyse Structurelle (A et non A) ---
        if child_kind == ExprKind::Not {
            let atom_id = child_node.children()[0];
            if positive_ids.contains(&atom_id) {
                return short_circuit_tautology(node_id, expr, kind);
            }
            negated_ids.insert(atom_id);
        } else {
            if negated_ids.contains(&child_id) {
                return short_circuit_tautology(node_id, expr, kind);
            }
            positive_ids.insert(child_id);
        }
    }

    Ok(false)
}

/// Reduces a parent node to a constant value when a structural tautology or contradiction is detected.
///
/// This helper is invoked when the simplification pass identifies both a formula $\phi$
/// and its negation $\neg \phi$ within the same collection of children.
///
/// # Logic
/// - For an **OR** node: $(\phi \lor \neg \phi \lor \dots)$ is a **Tautology**, reducing the node to `true`.
/// - For an **AND** node: $(\phi \land \neg \phi \land \dots)$ is a **Contradiction**, reducing the node to `false`.
///
/// # Arguments
/// * `node_id` - The ID of the parent node to be reduced.
/// * `logic` - A mutable reference to the expression arena.
/// * `kind` - The kind of the parent node (must be `And` or `Or`).
///
/// # Errors
/// Returns a [`ExprOpError`] if the node update fails in the underlying arena.
///
/// # Panics
/// Panics in debug/release if `kind` is not `ExprKind::And` or `ExprKind::Or`.
fn short_circuit_tautology(
    node_id: NodeId,
    expr: &mut Expr,
    kind: ExprKind
) -> Result<bool, ExprOpError> {
    match kind {
        // Law of excluded middle: (A ∨ ¬A) ≡ True
        ExprKind::Or => Ok(expr.set_to_bool(node_id, true)?),
        // Law of non-contradiction: (A ∧ ¬A) ≡ False
        ExprKind::And => Ok(expr.set_to_bool(node_id, false)?),
        _ => unreachable!("short_circuit_tautology called on non-logical gate: {:?}", kind),
    }
}

/// Reduces an AND/OR node that has exactly one child.
///
/// # Parameters
/// - `node_id`: the `NodeId` of the node to reduce. Only AND/OR nodes are considered.
/// - `logic`: mutable reference to the expression tree.
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
fn reduce_single_and_or_node(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprOpError> {
    // 1. On récupère l'ID de l'enfant unique sans bloquer l'emprunt mutable d'logic
    let single_child = {
        let node = expr.try_node(node_id)?;
        debug_assert!(node.kind() == ExprKind::And || node.kind() == ExprKind::Or);

        if node.children().len() != 1 {
            return Ok(false);
        }
        node.children()[0]
    };

    // 2. Le nœud 'node_id' est écrasé par le contenu de 'single_child'.
    // Puisque tous les parents pointent déjà vers 'node_id',
    // l'enfant remonte automatiquement d'un niveau dans l'arène.
    expr.move_to(single_child, node_id)?;

    Ok(true)
}

/// Simplifies an AND/OR node in a PDDL expression according to standard PDDL semantics.
///
/// # Parameters
/// - `node_id`: the `NodeId` of the node to simplification. Only AND/OR nodes are affected.
/// - `logic`: mutable reference to the `Expr` tree, used to access and mutate child nodes.
///
/// # Behavior
/// The function handles three main cases for AND/OR nodes:
///
/// 1. **Absorbing child**: If a child is an empty node of the **opposite typing**, the parent node
///    is replaced by the child's kind and its children are cleared.
///    - Example: `(and (or))` → becomes `(or)`
///    - Semantically, `(or)` with no children evaluates to `false`, `(and)` with no children evaluates to `true`.
///
/// 2. **Neutral child**: If a child is an empty node of the **same typing**, it is ignored and
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
fn simplify_empty_and_or_node(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprOpError> {
    // 1. On récupère les infos nécessaires sans bloquer l'arène
    let (node_kind, children) = {
        let node = expr.try_node(node_id)?;
        (node.kind(), node.children().to_vec())
    };

    let mut new_children = Vec::with_capacity(children.len());
    let mut modified = false;

    for child_id in children {
        let child = expr.try_node(child_id)?;

        // On ne s'intéresse qu'aux enfants "vides" (True/False potentiels)
        if child.children().is_empty() {
            let child_kind = child.kind();

            // CAS 1 : Élément Absorbant
            // (and ... false) -> false  |  (or ... true) -> true
            if (node_kind == ExprKind::And && child_kind == ExprKind::Or) ||
                (node_kind == ExprKind::Or && child_kind == ExprKind::And)
            {
                // Le parent node_id prend l'identité de l'enfant absorbant
                expr.move_to(child_id, node_id)?;
                return Ok(true);
            }

            // CAS 2 : Élément Neutre
            // (and ... true) -> (and ...) | (or ... false) -> (or ...)
            if child_kind == node_kind {
                modified = true;
                continue; // On ignore ce child_id, il ne sera pas dans new_children
            }
        }

        new_children.push(child_id);
    }

    // 2. Mise à jour si des éléments neutres ont été supprimés
    if modified {
        expr.try_node_mut(node_id)?.set_children(new_children);
    }

    Ok(modified)
}

/// Main function that merges `When` logic under an `And` or `Or` node
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
/// * `logic` - Mutable reference to the `Expr` tree being updated.
///
/// # Returns
///
/// * `Ok(true)` if any fusion occurred (an `Or` was created).
/// * `Ok(false)` if no fusion occurred (all `When`s had a single condition).
/// * `Err(ExprError)` if an error occurs during collection, allocation, or logic.
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
/// let fusion_occurred = merge_when_in_place(node_id, &mut logic)?;
/// if fusion_occurred {
///     println!("Some WHEN conditions were merged into OR nodes");
/// }
/// ```
pub fn merge_when(
    node_id: NodeId,
    expr: &mut Expr,
) -> Result<bool, ExprOpError> {
    // Collect non-WHEN children and merged WHEN conditions
    let (non_when, merged_map) = collect_and_merge_when(node_id, expr)?;

    // Rebuild the node's children and get whether a fusion occurred
    let fusion_occurred = rebuild_children_with_merged_when(node_id, non_when, merged_map, expr)?;

    // Return the fusion flag
    Ok(fusion_occurred)
}

/// Iterates over the children of a node, collects non-`When` children,
/// and merges `When` logic by their effect.
///
/// This function processes a logical `And` or `Or` node. It separates out the children
/// that are **not** `When` logic, and merges all `When` logic that have
/// the same effect. If multiple conditions share the same effect, they are grouped together.
///
/// # Arguments
///
/// * `node_id` - The ID of the parent node (`And` or `Or`) whose children are being processed.
/// * `logic` - A reference to the expression tree containing the node.
///
/// # Returns
///
/// * `Ok((non_when, merged_when))` where:
///     - `non_when` is a vector of NodeIds for children that are not `When` logic.
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
/// // Suppose node_id is an AND node containing WHEN logic
/// let (non_when, merged_when) = collect_and_merge_when(node_id, &logic)?;
/// // non_when contains all non-WHEN children
/// // merged_when groups WHEN conditions by effect
/// ```
fn collect_and_merge_when(
    node_id: NodeId,
    expr: &Expr
) -> Result<(Vec<NodeId>, Vec<(NodeId, Vec<NodeId>)>), ExprOpError> {
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

/// Rebuilds the children of a logical `And` or `Or` node by merging `When` logic.
/// Returns `true` if any fusion occurred (i.e., if multiple conditions were combined into an `Or`).
///
/// This function replaces `When` nodes with merged versions, combining conditions
/// that share the same effect. Non-`When` children are preserved as-is.
///
/// # Arguments
///
/// * `node_id` - The ID of the parent node (`And` or `Or`) whose children will be updated.
/// * `non_when` - A vector of NodeIds representing children that are **not** `When` logic.
/// * `merged_when` - A vector of tuples `(effect_node, conditions)` representing merged `When` logic.
///                   Each tuple contains the effect node ID and a vector of condition node IDs.
/// * `logic` - Mutable reference to the `Expr` tree being updated.
///
/// # Returns
///
/// * `Ok(true)` if a fusion occurred (an `Or` node was created for multiple conditions).
/// * `Ok(false)` if no fusion occurred (all `When` nodes had a single condition).
/// * `Err(ExprError)` if an error occurs during node allocation or logic.
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
/// let fusion = rebuild_children_with_merged_when(node_id, non_when_vec, merged_when_vec, &mut logic)?;
/// if fusion {
///     println!("Some WHEN conditions were merged into OR nodes");
/// }
/// ```
fn rebuild_children_with_merged_when(
    node_id: NodeId,
    non_when: Vec<NodeId>,
    merged_when: Vec<(NodeId, Vec<NodeId>)>,
    expr: &mut Expr,
) -> Result<bool, ExprOpError> {
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
            simplify(or_node, expr)?; // Simplify OR node
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
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;

    /// Realistic test: AND root with nested AND/OR children, duplicates, and empty OR
    ///
    /// Input: (and (A) (and (B) (C) (B)) (or) (and (C)))
    /// Expected: (or) <- empty OR inside AND makes the root OR
    #[test]
    fn test_realistic_and_flatten() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and A (and B C B) (or) (and C))
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        let inner1 = builder.and(vec![b, c, b]); // Duplicated B
        let inner2 = builder.and(vec![c]);       // Single C
        let empty_or = builder.or(vec![]);       // Empty OR

        let root = builder.and(vec![a, inner1, empty_or, inner2]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation
        // Using try_root_id() to propagate potential errors
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // According to your original assert, the expected result is an empty OR
        assert_eq!(expr.kind(), Some(ExprKind::Or));

        let root_node = expr.try_root_node()?;
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Realistic test: OR root with an empty AND child
    ///
    /// Input: (or (or (A)) (and) (B) (or))
    /// Expected: (and) <- empty AND absorbs the OR root
    #[test]
    fn test_realistic_or_flatten() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or (or A) (and) B (or))
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);

        let inner_or1 = builder.or(vec![a]);
        let empty_and = builder.and(vec![]);
        let empty_or2 = builder.or(vec![]);

        let root = builder.or(vec![inner_or1, empty_and, b, empty_or2]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The expected result is an empty AND (logically "True")
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        assert!(root_node.children().is_empty());

        Ok(())
    }
}


#[cfg(test)]
mod flatten_and_or_node_tests {
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use super::*;

    /// Test flattening a root AND node with nested AND children.
    ///
    /// Input: (and (A) (and (B) (C)) (D))
    /// Expected: (and (A) (B) (C) (D))
    #[test]
    fn test_flatten_root_and() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and A (and B C) D)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);
        let d = builder.atomic_formula(4, vec![]);

        let inner_and = builder.and(vec![b, c]);
        let root = builder.and(vec![a, inner_and, d]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Flatten nested ANDs
        flatten_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        // Expecting 4 children: A, B, C, and D
        assert_eq!(root_node.children().len(), 4);

        // Verify children kinds
        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }

    /// Test flattening a root OR node with nested OR children.
    ///
    /// Input: (or (A) (or (B) (C)))
    /// Expected: (or (A) (B) (C))
    #[test]
    fn test_flatten_root_or() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or A (or B C))
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);

        let inner_or = builder.or(vec![b, c]);
        let root = builder.or(vec![a, inner_or]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Flatten nested ORs
        flatten_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::Or));

        let root_node = expr.try_root_node()?;
        // Expecting 3 children: A, B, and C
        assert_eq!(root_node.children().len(), 3);

        // Verify all children are atoms
        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }

    /// Test flattening nested AND children in a non-trivial root.
    ///
    /// Input: (and (A) (and (B) (C)) (D))
    /// Expected: (and (A) (B) (C) (D))
    #[test]
    fn test_flatten_non_root_and() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and A (and B C) D)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let c = builder.atomic_formula(3, vec![]);
        let d = builder.atomic_formula(4, vec![]);

        let inner_and = builder.and(vec![b, c]);
        let root = builder.and(vec![a, inner_and, d]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Flatten specifically at the root ID
        flatten_and_or_node(root, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        // Children should be flattened to: [A, B, C, D]
        assert_eq!(root_node.children().len(), 4);

        // Verify all flattened children are atomic
        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }

    /// Test that a node with no nested AND/OR is unchanged.
    ///
    /// Input: (and (A) (B))
    /// Expected: (and (A) (B))
    #[test]
    fn test_no_nested_nodes() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and A B) - No nested ANDs
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let root = builder.and(vec![a, b]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Flattening should result in no change
        flatten_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        // Children count should remain 2
        assert_eq!(root_node.children().len(), 2);

        // Verify children are still the original atoms
        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }
}

#[cfg(test)]
mod deduplicate_and_or_node_tests {
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use super::*;

    /// Test NodeId-based deduplication in a root AND node.
    ///
    /// Input: (and (A) (A) (B))
    /// Expected: (and (A) (B))
    #[test]
    fn test_root_and_nodeid_duplicates() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and A A B)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);

        // We use 'a' twice in the same AND node
        let root = builder.and(vec![a, a, b]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Remove duplicate children
        deduplicate_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        // Children should be reduced to just [A, B]
        assert_eq!(root_node.children().len(), 2);

        // Verify the unique children are what we expect
        assert_eq!(expr.get_node_kind(root_node.children()[0]), Some(ExprKind::AtomicFormula));
        assert_eq!(expr.get_node_kind(root_node.children()[1]), Some(ExprKind::AtomicFormula));

        Ok(())
    }

    /// Test NodeId-based deduplication in a root OR node.
    ///
    /// Input: (or (A) (B) (B))
    /// Expected: (or (A) (B))
    #[test]
    fn test_root_or_nodeid_duplicates() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or A B B)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);

        // Using 'b' twice in the same OR node
        let root = builder.or(vec![a, b, b]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Remove duplicate children from the OR node
        deduplicate_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::Or));

        let root_node = expr.try_root_node()?;
        // Children should be reduced to [A, B]
        assert_eq!(root_node.children().len(), 2);

        // Verify individual children are atomic formulas
        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }

    /// Test NodeId-based deduplication preserves non-duplicated children.
    ///
    /// Input: (and (A) (B))
    /// Expected: (and (A) (B))
    #[test]
    fn test_and_no_duplicates() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and A B) - No duplicates present
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let root = builder.and(vec![a, b]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Deduplicate (should be a no-op)
        deduplicate_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        // Children count should still be 2
        assert_eq!(root_node.children().len(), 2);

        // Verify original order/existence
        assert_eq!(expr.get_node_kind(root_node.children()[0]), Some(ExprKind::AtomicFormula));
        assert_eq!(expr.get_node_kind(root_node.children()[1]), Some(ExprKind::AtomicFormula));

        Ok(())
    }

    /// Test structural deduplication in a root AND node with duplicate subtrees.
    ///
    /// Input: (and (and (A) (B)) (and (A) (B)) (C))
    /// Expected: (and (and (A) (B)) (C))
    #[test]
    fn test_root_and_structural_duplicates() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and (and A B) (and A B) C)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);

        // Create two structurally identical but distinct nodes
        let inner1 = builder.and(vec![a, b]);
        let inner2 = builder.and(vec![a, b]);

        let c = builder.atomic_formula(3, vec![]);
        let root = builder.and(vec![inner1, inner2, c]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Deduplicate children based on structure
        deduplicate_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        // Children should be reduced to [(and A B), C]
        assert_eq!(root_node.children().len(), 2);

        // Verify first child is the nested AND
        let first_child = root_node.children()[0];
        assert_eq!(expr.get_node_kind(first_child), Some(ExprKind::And));

        // Verify second child is the atomic formula C
        let second_child = root_node.children()[1];
        assert_eq!(expr.get_node_kind(second_child), Some(ExprKind::AtomicFormula));

        Ok(())
    }

    /// Test structural deduplication in a root OR node with duplicate subtrees.
    ///
    /// Input: (or (or (A) (B)) (or (A) (B)) (C))
    /// Expected: (or (or (A) (B)) (C))
    #[test]
    fn test_root_or_structural_duplicates() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or (or A B) (or A B) C)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);

        // Two different nodes representing the same structure
        let inner1 = builder.or(vec![a, b]);
        let inner2 = builder.or(vec![a, b]);

        let c = builder.atomic_formula(3, vec![]);
        let root = builder.or(vec![inner1, inner2, c]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Structural deduplication
        deduplicate_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::Or));

        let root_node = expr.try_root_node()?;
        // Expecting [(or A B), C]
        assert_eq!(root_node.children().len(), 2);

        // Verify the first child is the remaining OR branch
        let first_child = root_node.children()[0];
        assert_eq!(expr.get_node_kind(first_child), Some(ExprKind::Or));

        // Verify the second child is the atom C
        let second_child = root_node.children()[1];
        assert_eq!(expr.get_node_kind(second_child), Some(ExprKind::AtomicFormula));

        Ok(())
    }

}

#[cfg(test)]
mod simplify_tautologies_and_contradictions_tests {
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use super::*;

    /// Input: (or A (not A))
    /// Expected output: (and)  // tautology in OR -> true
    #[test]
    fn test_or_tautology() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or A (not A))
        let a = builder.atomic_formula(1, vec![]);
        let not_a = builder.not(a);
        let root = builder.or(vec![a, not_a]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify tautologies
        let changed = simplify_tautologies_and_contradictions(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert!(changed, "The expression should have been simplified");

        // The result should be "True", represented as an empty AND node
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        assert!(root_node.children().is_empty(), "True should be an empty AND node");

        Ok(())
    }

    /// Input: (and A (not A))
    /// Expected output: (or)  // contradiction in AND -> false
    #[test]
    fn test_and_contradiction() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and A (not A))
        let a = builder.atomic_formula(1, vec![]);
        let not_a = builder.not(a);
        let root = builder.and(vec![a, not_a]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify contradictions
        let changed = simplify_tautologies_and_contradictions(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert!(changed, "The contradiction should have been detected and simplified");

        // The result should be "False", represented as an empty OR node
        assert_eq!(expr.kind(), Some(ExprKind::Or));

        let root_node = expr.try_root_node()?;
        assert!(root_node.children().is_empty(), "False should be an empty OR node");

        Ok(())
    }

    /// Input: (or A B)  // no tautology
    /// Expected output: unchanged
    #[test]
    fn test_or_no_tautology() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or A B) - No tautology here
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let root = builder.or(vec![a, b]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: simplification (should result in no change)
        let changed = simplify_tautologies_and_contradictions(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert!(!changed, "The expression should not have changed");

        // Root should still be the same OR node
        assert_eq!(expr.kind(), Some(ExprKind::Or));

        let root_node = expr.try_root_node()?;
        assert_eq!(root_node.children().len(), 2);

        Ok(())
    }

    /// Input: (and A B)  // no contradiction
    /// Expected output: unchanged
    #[test]
    fn test_and_no_contradiction() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and A B) - No contradiction here
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let root = builder.and(vec![a, b]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: simplification (should result in no change)
        let changed = simplify_tautologies_and_contradictions(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert!(!changed, "The expression should not have been flagged as changed");

        // The root should still be the original AND node
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        assert_eq!(root_node.children().len(), 2);

        Ok(())
    }
}

#[cfg(test)]
mod reduce_single_and_or_node_tests {
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use super::*;
    /// Test root AND with a single child
    ///
    /// Input: (and A)
    /// Expected: A
    #[test]
    fn test_root_and_single_child() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and A)
        // Using atomic_formula as it's the standard leaf in your previous examples
        let a = builder.atomic_formula(1, vec![]);
        let root = builder.and(vec![a]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Reduce single-child junctions
        // Note: This ops typically updates the parent's pointer or replaces the root
        reduce_single_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The root should now be the AtomicFormula itself, not the AND node
        assert_eq!(expr.kind(), Some(ExprKind::AtomicFormula));

        // Ensure the root ID now points to a node of the correct kind
        let root_node = expr.try_root_node()?;
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);

        Ok(())
    }
    /// Test non-root AND with a single child
    ///
    /// Input: (and (and A))
    /// Expected: (and A)
    #[test]
    fn test_and_single_child_non_root() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and (and A))
        let a = builder.atomic_formula(1, vec![]);
        let inner_and = builder.and(vec![a]);
        let root = builder.and(vec![inner_and]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Reduce the internal single-child AND
        reduce_single_and_or_node(inner_and, &mut expr)?;

        // 3. Validation
        let root_id = expr.try_root_id()?;
        let root_node = expr.try_node(root_id)?;

        // The outer root should still be an AND node
        assert_eq!(root_node.kind(), ExprKind::And);

        // But its child should now be the AtomicFormula directly
        assert_eq!(root_node.children().len(), 1);
        let child_id = root_node.children()[0];
        assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));

        Ok(())
    }

    /// Test root OR with a single child
    ///
    /// Input: (or A)
    /// Expected: A
    #[test]
    fn test_root_or_single_child() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or A)
        let a = builder.atomic_formula(1, vec![]);
        let root = builder.or(vec![a]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify unary OR
        reduce_single_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The OR wrapper should be gone; the root should now be the AtomicFormula
        assert_eq!(expr.kind(), Some(ExprKind::AtomicFormula));

        let root_node = expr.try_root_node()?;
        assert_eq!(root_node.kind(), ExprKind::AtomicFormula);

        Ok(())
    }
    /// Test non-root OR with a single child
    ///
    /// Input: (or (or A))
    /// Expected: (or A)
    #[test]
    fn test_or_single_child_non_root() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or (or A))
        let a = builder.atomic_formula(1, vec![]);
        let inner_or = builder.or(vec![a]);
        let root = builder.or(vec![inner_or]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Reduce the internal single-child OR
        reduce_single_and_or_node(inner_or, &mut expr)?;

        // 3. Validation
        let root_node = expr.try_root_node()?;

        // The outer root should remain an OR node
        assert_eq!(root_node.kind(), ExprKind::Or);

        // Its child should now be the AtomicFormula directly, bypassing the inner OR
        assert_eq!(root_node.children().len(), 1);
        let child_id = root_node.children()[0];
        assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));

        Ok(())
    }
    /// Test that nodes with multiple children are unchanged
    ///
    /// Input: (and A B)
    /// Expected: (and A B)
    #[test]
    fn test_and_multiple_children() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and A B) - Multiple children
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let root = builder.and(vec![a, b]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Attempt reduction
        reduce_single_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The root should remain an AND node because it has > 1 child
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        assert_eq!(root_node.children().len(), 2);

        Ok(())
    }
}

#[cfg(test)]
mod simplify_empty_and_or_node_tests {
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::expr::ExprKind;
    use super::*;

    /// Test that an AND node with an empty AND child removes the empty child.
    ///
    /// Input: (and (and))
    /// Expected: (and)
    #[test]
    fn test_and_with_empty_and_child() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and (and))
        // The inner (and) is logically "True"
        let empty_and = builder.and(vec![]);
        let root = builder.and(vec![empty_and]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify empty junctions within their own kind
        simplify_empty_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        // The redundant empty_and child should have been removed
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test absorbing child: AND node with an empty OR child.
    ///
    /// Input: (and (or))
    /// Expected: (or)
    #[test]
    fn test_and_absorbing_or() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and (or))
        // (or) is False, so (and False) should become False
        let empty_or = builder.or(vec![]);
        let root = builder.and(vec![empty_or]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify empty junctions acting as absorbers
        simplify_empty_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The result should be "False", represented as an empty OR node
        assert_eq!(expr.kind(), Some(ExprKind::Or));

        let root_node = expr.try_root_node()?;
        assert!(root_node.children().is_empty());

        Ok(())
    }

    /// Test that non-empty children of AND are preserved.
    ///
    /// Input: (and (A) (B))
    /// Expected: (and (A) (B))
    #[test]
    fn test_and_keep_children() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and A B)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let root = builder.and(vec![a, b]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify empty nodes (should be a no-op here)
        simplify_empty_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        // Both A and B should still be present
        assert_eq!(root_node.children().len(), 2);

        // Verify children are still AtomicFormulas
        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }

    /// Test mixed children in AND: empty AND, empty OR, and an atomic formula.
    ///
    /// Input: (and (and) (or) (A))
    /// Expected: (or)
    #[test]
    fn test_and_mixed_children_absorb() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and (and) (or) A)
        // (and) is True (Identity)
        // (or)  is False (Absorber for AND)
        let empty_and = builder.and(vec![]);
        let empty_or = builder.or(vec![]);
        let a = builder.atomic_formula(1, vec![]);

        let root = builder.and(vec![empty_and, empty_or, a]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify empty junctions
        simplify_empty_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The absorbing element (or) must dominate, turning the whole thing into (or)
        assert_eq!(expr.kind(), Some(ExprKind::Or));

        let root_node = expr.try_root_node()?;
        assert!(root_node.children().is_empty(), "Result should be an empty OR node (False)");

        Ok(())
    }

    /// Test that an OR node with an empty OR child removes the empty child.
    ///
    /// Input: (or (or))
    /// Expected: (or)
    #[test]
    fn test_or_with_empty_or_child() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or (or))
        // The inner (or) is logically "False" (the identity for OR)
        let empty_or = builder.or(vec![]);
        let root = builder.or(vec![empty_or]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify empty junctions of the same kind
        simplify_empty_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::Or));

        let root_node = expr.try_root_node()?;
        // The redundant inner 'or' should have been removed
        assert!(root_node.children().is_empty(), "The result should be a clean, empty OR node");

        Ok(())
    }

    /// Test absorbing child: OR node with an empty AND child.
    ///
    /// Input: (or (and))
    /// Expected: (and)
    #[test]
    fn test_or_absorbing_and() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or (and))
        // (and) is True, and (A or True) is always True.
        let empty_and = builder.and(vec![]);
        let root = builder.or(vec![empty_and]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: The OR node should be "absorbed" by the True child
        simplify_empty_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The result should be "True", represented as an empty AND node
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        assert!(root_node.children().is_empty(), "Result should be an empty AND node");

        Ok(())
    }

    /// Test that non-empty children of OR are preserved.
    ///
    /// Input: (or (A) (B))
    /// Expected: (or (A) (B))
    #[test]
    fn test_or_keep_children() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or A B)
        let a = builder.atomic_formula(1, vec![]);
        let b = builder.atomic_formula(2, vec![]);
        let root = builder.or(vec![a, b]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Check for empty-node simplifications
        simplify_empty_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        assert_eq!(expr.kind(), Some(ExprKind::Or));

        let root_node = expr.try_root_node()?;
        // The nodes A and B must be preserved
        assert_eq!(root_node.children().len(), 2);

        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::AtomicFormula));
        }

        Ok(())
    }

    /// Test mixed children in OR: empty OR, empty AND, and an atomic formula.
    ///
    /// Input: (or (or) (and) (A))
    /// Expected: (and)
    #[test]
    fn test_or_mixed_children_absorb() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (or (or) (and) A)
        // (or)  is False (Identity for OR)
        // (and) is True  (Absorber for OR)
        let empty_or = builder.or(vec![]);
        let empty_and = builder.and(vec![]);
        let a = builder.atomic_formula(1, vec![]);

        let root = builder.or(vec![empty_or, empty_and, a]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify empty nodes
        simplify_empty_and_or_node(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The True constant (empty AND) must absorb the OR node
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        assert!(root_node.children().is_empty(), "Result should be an empty AND node (True)");

        Ok(())
    }

    /// Test fusion of multiple WHENs with the same effect under an AND node.
    /// Input: (and (when C1 E) (when C2 E))
    /// Expected: (when (or C1 C2) E)
    #[test]
    fn test_when_merge_same_effect() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and (when C1 E) (when C2 E))
        let c1 = builder.atomic_formula(1, vec![]);
        let c2 = builder.atomic_formula(2, vec![]);
        let e  = builder.atomic_formula(3, vec![]);

        let w1 = builder.when(c1, e);
        let w2 = builder.when(c2, e);

        let root = builder.and(vec![w1, w2]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Factor out common effect E
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The root should now be a single WHEN node: (when (or C1 C2) E)
        assert_eq!(expr.kind(), Some(ExprKind::When));

        let root_node = expr.try_root_node()?;
        assert_eq!(root_node.children().len(), 2);

        // Child 0: The combined conditions (OR C1 C2)
        let cond_id = root_node.children()[0];
        assert_eq!(expr.get_node_kind(cond_id), Some(ExprKind::Or));
        assert_eq!(expr.try_node(cond_id)?.children().len(), 2);

        // Child 1: The factored effect E
        let eff_id = root_node.children()[1];
        assert_eq!(expr.get_node_kind(eff_id), Some(ExprKind::AtomicFormula));

        Ok(())
    }


    /// Test that WHENs with different effects are not merged.
    /// Input: (and (when C1 E1) (when C2 E2))
    /// Expected: (and (when C1 E1) (when C2 E2))
    #[test]
    fn test_when_not_merge_different_effect() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and (when C1 E1) (when C2 E2))
        let c1 = builder.atomic_formula(1, vec![]);
        let c2 = builder.atomic_formula(2, vec![]);
        let e1 = builder.atomic_formula(3, vec![]);
        let e2 = builder.atomic_formula(4, vec![]);

        let w1 = builder.when(c1, e1);
        let w2 = builder.when(c2, e2);

        let root = builder.and(vec![w1, w2]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Attempt to simplification
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The root must remain an AND node with two distinct WHEN children
        assert_eq!(expr.kind(), Some(ExprKind::And));

        let root_node = expr.try_root_node()?;
        assert_eq!(root_node.children().len(), 2);

        for &child_id in root_node.children() {
            assert_eq!(expr.get_node_kind(child_id), Some(ExprKind::When));
        }

        Ok(())
    }

    /// Test WHEN with a single condition is preserved as-is (no OR created).
    /// Input: (and (when C E))
    /// Expected: (when C E)
    #[test]
    fn test_when_single_condition() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and (when C E))
        let c = builder.atomic_formula(1, vec![]);
        let e = builder.atomic_formula(2, vec![]);

        let w = builder.when(c, e);
        let root = builder.and(vec![w]);

        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Simplify the expression
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The root AND should be reduced, making the WHEN node the new root
        assert_eq!(expr.kind(), Some(ExprKind::When));

        let root_node = expr.try_root_node()?;
        assert_eq!(root_node.children().len(), 2);

        // Verify internal structure of the WHEN node is preserved
        let cond_id = root_node.children()[0];
        let eff_id = root_node.children()[1];

        assert_eq!(expr.get_node_kind(cond_id), Some(ExprKind::AtomicFormula));
        assert_eq!(expr.get_node_kind(eff_id), Some(ExprKind::AtomicFormula));

        Ok(())
    }

    /// Test fusion of identical WHEN conditions under an AND node.
    /// Input: (and (when C E) (when C E))
    /// Expected: (when C E) after OR simplification
    #[test]
    fn test_when_merge_with_or_simplification() -> Result<(), ExprOpError> {
        let mut builder = ExprBuilder::new();

        // 1. Setup: (and (when C E) (when C E))
        let c = builder.atomic_formula(1, vec![]);
        let e = builder.atomic_formula(2, vec![]);

        let w1 = builder.when(c, e);
        let w2 = builder.when(c, e);

        let root = builder.and(vec![w1, w2]);
        builder.set_root(root)?;
        let mut expr = builder.finish();

        // 2. Transformation: Factorize and then Deduplicate
        simplify(expr.try_root_id()?, &mut expr)?;

        // 3. Validation
        // The result should not be (when (or C C) E), but the fully reduced (when C E)
        assert_eq!(expr.kind(), Some(ExprKind::When));

        let root_node = expr.try_root_node()?;

        // The condition child must be the AtomicFormula directly (the OR was pruned)
        let cond_id = root_node.children()[0];
        assert_eq!(expr.get_node_kind(cond_id), Some(ExprKind::AtomicFormula));

        // The effect child remains the AtomicFormula
        let eff_id = root_node.children()[1];
        assert_eq!(expr.get_node_kind(eff_id), Some(ExprKind::AtomicFormula));

        Ok(())
    }

}
