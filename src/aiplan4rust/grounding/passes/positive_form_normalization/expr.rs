use crate::aiplan4rust::lang::AtomSkeletonId;
use crate::aiplan4rust::lir::old::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::old::expr::{Expr, ExprContent, ExprKind};
use crate::aiplan4rust::tree::NodeId;

/// Encodes logical negations into atomic predicate identifiers using a bit-mask (MSB).
///
/// This function performs the final lowering of the expression tree into a
/// Positive Normal Form (PNF) suitable for the Grounder and Datalog engine.
/// It replaces `Not(AtomicFormula)` structures with a single `AtomicFormula`
/// node where the internal `AtomSkeletonId` is marked as negated.
///
/// # Preconditions
/// This function acts as a **strict final pass** in the simplification pipeline. It assumes
/// the tree has been pre-processed:
/// 1. **Implication Elimination**: All `Imply` nodes must be removed.
/// 2. **Negation Propagation**: All `Not` nodes must be pushed down to the leaves.
///    A `Not` node should only ever have an `AtomicFormula` or a `Comparison` as its child.
/// 3. **Simplification**: Double negations (`NOT NOT`) must be simplified out.
///
/// # Parameters
/// * `node_id`: The root [`NodeId`] from which to start the encoding traversal.
/// * `logic`: A mutable reference to the [`Expr`] arena for in-place tree mutation.
/// * `negated_atoms`: A mutable reference to a caller-owned [`Vec`]. This vector will be
///   populated with the [`AtomSkeletonId`] of every atom that gets negated during this pass.
/// * `stack`: A mutable reference to a caller-owned [`Vec<NodeId>`] used as a scratchpad
///   for DFS traversal. This avoids heap allocations during the traversal.
///
/// # Transformation Logic
/// - **`Not(AtomicFormula)`**: Handled by `handle_not_node`. Typically, the `Not` node is
///   absorbed, and the child `AtomicFormula` is moved into its place with its MSB
///   (Most Significant Bit) set to true.
/// - **`Not(Comparison)`**: Preserved as a structural `Not` node (if `handle_not_node`
///   returns `false`), as comparisons are handled during evaluation.
/// - **Logical/Temporal Containers**: `And`, `Or`, `Forall`, `Exists`, `Always`, etc.,
///   are traversed to find nested negations.
///
/// # Optimization: Zero-Allocation Traversal
/// - The DFS traversal uses the provided `stack` buffer, ensuring **zero heap allocations**
///   during the process if the buffer has sufficient capacity.
/// - The traversal stops descending at terminal nodes (Atoms, Comparisons) to save cycles.
/// - The use of `sort_unstable()` and `dedup()` at the end of the pass ensures the
///   `negated_atoms` buffer remains compact.
///
/// # Post-processing
/// The `negated_atoms` vector is populated, sorted, and deduplicated internally at
/// the end of each call. If multiple calls are made (e.g., across different actions),
/// the caller should perform a final global `.sort_unstable()` and `.dedup()` on the
/// combined results.
///
/// # Errors
/// Returns [`ExprOpError::invalid_expr_node`] if:
/// - A `Not` node is malformed or found above an invalid kind (e.g., `And`, `Or`).
/// - An `Imply` node is encountered anywhere (Strictness violation).
/// - An `AtomicFormula` is found under a `Not` but is already negated at the bit level.
pub fn to_pnf(
    node_id: NodeId,
    expr: &mut Expr,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    stack: &mut Vec<(NodeId, bool)>, // La pile transporte (ID, est_une_condition)
    is_effect: bool,                 // Indique si on démarre dans un arbre d'effets
) -> Result<(), ExprOpError> {
    stack.clear();

    // Si on est dans un arbre d'effets, on commence en mode "non-condition" (false).
    // Si on est dans les préconditions, tout est considéré comme une condition (true).
    stack.push((node_id, !is_effect));

    while let Some((curr_id, in_condition)) = stack.pop() {
        let node_kind = expr.try_node(curr_id)?.kind();

        match node_kind {
            ExprKind::Not => {
                // IMPORTANT : On ne traite le NOT que si on est dans un contexte logique (condition).
                // Si in_condition est false, c'est un "Delete Effect", on l'ignore (Delete-Relaxation).
                if in_condition {
                    if handle_not_node(curr_id, expr, negated_atoms)? {
                        continue;
                    }
                }
                // Si c'est une comparaison ou un effet de suppression, on s'arrête là.
                continue;
            }

            ExprKind::When => {
                let node = expr.try_node(curr_id)?;
                let children = node.children();
                if children.len() != 2 {
                    return Err(ExprOpError::invalid_expr_node(curr_id, node_kind));
                }
                // Enfant 0 : La Condition -> On passe in_condition à TRUE
                stack.push((children[0], true));
                // Enfant 1 : L'Effet -> On passe in_condition à FALSE
                stack.push((children[1], false));
            }

            // Connecteurs logiques et temporels
            ExprKind::And
            | ExprKind::Or
            | ExprKind::Forall
            | ExprKind::Exists
            | ExprKind::Always
            | ExprKind::Sometime
            | ExprKind::Within
            | ExprKind::AtMostOnce
            | ExprKind::SometimeAfter
            | ExprKind::SometimeBefore
            | ExprKind::AlwaysWithin
            | ExprKind::HoldDuring
            | ExprKind::HoldAfter => {
                let node = expr.try_node(curr_id)?;
                // On propage le contexte actuel (in_condition) aux enfants
                for &child_id in node.children().iter().rev() {
                    stack.push((child_id, in_condition));
                }
            }

            ExprKind::Imply => {
                return Err(ExprOpError::invalid_expr_node(curr_id, node_kind));
            }

            _ => {}
        }
    }

    negated_atoms.sort_unstable();
    negated_atoms.dedup();
    Ok(())
}

/// Processes a `Not` node by attempting to absorb the negation into its child
/// and collecting the resulting negated atom identifier.
///
/// This function implements the core "lowering" logic of the PNF (Prenex Normal Form)
/// encoding phase. It performs an in-place mutation of the expression arena.
///
/// # Parameters
/// * `curr_id`: The [`NodeId`] of the `Not` node currently being processed.
/// * `logic`: A mutable reference to the [`Expr`] arena. This allows the function
///   to move the child's content into the current node's slot (absorption).
/// * `negated_atoms`: A mutable reference to a caller-owned [`Vec`]. When an
///   `AtomicFormula` is successfully negated via MSB-masking, its updated
///   [`AtomSkeletonId`] is pushed onto this vector for indexing/CWA purposes.
///
/// # Transformation Logic
/// 1. **AtomicFormula**: The negation is absorbed. The `Not` node is transformed
///    into an `AtomicFormula` with its internal negation bit set to true.
///    The resulting ID is recorded in `negated_atoms`.
/// 2. **Comparison**: The negation remains as a structural node above the
///    comparison. No bit-flip is applied, and traversal stops for this branch.
///
/// # Side Effects
/// - Modifies the `logic` arena (node replacement).
/// - Appends the negated [`AtomSkeletonId`] to the `negated_atoms` vector.
///
/// # Errors
/// Returns [`ExprOpError::invalid_expr_node`] if:
/// * The `Not` node is empty (no children).
/// * A double negation is detected (structural `Not` under a `Not`).
/// * A complex logical connector or quantifier is found under the `Not`.
/// * The child is an `AtomicFormula` that is already marked as negated.
///
/// # Returns
/// * `Ok(true)` if the child was an atom and was absorbed (it's now a leaf).
/// * `Ok(false)` if the child was a comparison (the `Not` node is preserved).
#[inline(always)]
fn handle_not_node(
    node_id: NodeId,
    expr: &mut Expr,
    negated_atoms: &mut Vec<AtomSkeletonId>,
) -> Result<bool, ExprOpError> {
    let node_kind = ExprKind::Not;

    // 1. Safely extract the child ID.
    // Using a scope to drop the immutable borrow of 'logic' immediately,
    // allowing subsequent mutations.
    let child_id = {
        let node = expr.try_node(node_id)?;
        node.children()
            .first()
            .copied()
            .ok_or_else(|| ExprOpError::invalid_expr_node(node_id, node_kind))?
    };

    let child_kind = expr.try_node(child_id)?.kind();

    match child_kind {
        // Double negation should have been removed by a previous simplification pass.
        ExprKind::Not => Err(ExprOpError::invalid_expr_node(child_id, child_kind)),

        ExprKind::AtomicFormula => {
            // Check the current negation state of the atom.
            // PNF encoding requires that we don't flip a bit that is already set (logic error).
            let is_negated =
                if let ExprContent::AtomSkeleton(id) = expr.try_node(child_id)?.content() {
                    id.is_negated()
                } else {
                    // Ensure the node content matches its kind.
                    return Err(ExprOpError::invalid_expr_node(child_id, child_kind));
                };

            if is_negated {
                return Err(ExprOpError::invalid_expr_node(child_id, child_kind));
            }

            // 2. Absorption Phase: Move the AtomicFormula data into the 'Not' node's slot.
            // This effectively deletes the child and transforms the 'Not' node into an 'AtomicFormula'.
            expr.move_to(child_id, node_id)?;

            // 3. Bit Flip: Set the MSB (Most Significant Bit) of the AtomSkeletonId to true.
            // This represents the logical negation within the atom itself.
            let node_mut = expr.try_node_mut(node_id)?;
            if let ExprContent::AtomSkeleton(ref mut id) = node_mut.content_mut() {
                id.set_negated(true);
                negated_atoms.push(*id);
            }

            // Return true to signal the atom was absorbed (stop descending this branch).
            Ok(true)
        }

        // Comparison nodes remain under a 'Not'.
        // We stop the traversal here because comparison children are terms, not formulas.
        ExprKind::Comparison => Ok(false),

        // Any other node (And, Or, Quantifiers) under a 'Not' is a pipeline violation.
        _ => Err(ExprOpError::invalid_expr_node(child_id, child_kind)),
    }
}
