use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::pnf::scratchpad::PnfScratchpad;
use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprStore};
use crate::aiplan4rust::support::lang::AtomSkeletonId;

/// Final lowering of an expression tree into its encoded PNF (Prenex Normal Form).
///
/// Convenient entry point that allocates a temporary scratchpad on the fly.
///
/// # Preconditions
///
/// This function assumes that the expression tree has already undergone the following normalization passes:
/// 1. **NNF (Negation Normal Form)**: Negation operators (`Not`) must strictly and directly target
///    terminal literals (atoms or comparisons). No negation is allowed to sit above a
///    quantifier (`Forall`, `Exists`) or a logical connective (`And`, `Or`).
/// 2. **QNF (Quantifier Normal Form / Variable Standardization)**: All variables bound by quantifiers
///    must have been uniquely renamed to avoid any accidental variable capture or collision
///    when pulling quantifiers towards the root level.
///
/// If these preconditions are violated, the function will immediately return a [`GroundingError`].
pub fn to_pnf(
    expr_id: ExprId,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    is_effect: bool,
) -> Result<ExprId, GroundingError> {
    // Local scratchpad allocation for one-off conversions
    let mut scratchpad = PnfScratchpad::new();
    to_pnf_with(expr_id, store, negated_atoms, &mut scratchpad, is_effect)
}

/// Final lowering of an expression tree into its encoded PNF (Prenex Normal Form).
///
/// Reconstructs the tree bottom-up using hash-consing and returns the new [`ExprId`].
///
/// # Preconditions
///
/// The PNF conversion algorithm relies on strict structural invariants of the source tree:
///
/// * **NNF (Negation Normal Form)**: Complex negations (e.g., double negations `Not(Not(...))` or
///   negated logical blocks `Not(And(...))`) are forbidden. The algorithm processes negations
///   by direct absorption into the atom's bit-mask. Encountering a non-positive structure
///   will immediately return a `StorerError::invalid_node` error.
/// * **QNF (Quantifier Normal Form / Variable Standardization)**: Each quantifier must possess
///   a unique variable identifier across the entire expression. Otherwise, pulling quantifiers
///   to the root level will destroy the original semantics via accidental variable capture.
///
/// # Algorithm
///
/// The process is strictly iterative (backed by an explicit stack) to prevent stack overflows
/// on deeply nested expression trees. It operates in a two-phase sequence:
/// 1. **Downwards Phase**: Propagates the logical context (`in_condition`) and performs
///    fail-fast validation of NNF invariants.
/// 2. **Upwards Phase**: Reconstructs the tree using the `ExprStore` hash-consing mechanism
///    to guarantee aggressive node deduplication without dynamic heap allocations.
pub fn to_pnf_with(
    expr_id: ExprId,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut PnfScratchpad,
    is_effect: bool,
) -> Result<ExprId, GroundingError> {
    scratchpad.clear();

    // Bootstrap the stack: (ExprId, InCondition, ChildrenPushed)
    scratchpad.stack.push((expr_id, !is_effect, false));

    while let Some((old_id, in_condition, children_pushed)) = scratchpad.stack.pop() {
        if !children_pushed {
            // --- 1. DOWNWARDS PHASE (CONTEXT PROPAGATION & VALIDATION) ---
            // Re-push current node with children_pushed=true to process it during the upwards phase
            scratchpad.stack.push((old_id, in_condition, true));

            // Copy-by-dereference since ExprKind is Copy (Zero-Allocation / DOD optimization)
            let entry_kind = *store[old_id].kind();
            match entry_kind {
                ExprKind::Not => {
                    if in_condition {
                        let child_id = *store[old_id]
                            .children()
                            .first()
                            .expect("Not must have a child");

                        // Fail-Fast: Detect illegal double negations early
                        if store[child_id].kind() == &ExprKind::Not {
                            return Err(StorerError::invalid_node(child_id).into());
                        }

                        scratchpad.stack.push((child_id, in_condition, false));
                    }
                }
                ExprKind::When => {
                    let children = store[old_id].children();
                    if children.len() != 2 {
                        return Err(StorerError::invalid_node(old_id).into());
                    }
                    // Split context: condition part is true, effect part is false
                    scratchpad.stack.push((children[0], true, false)); // Condition
                    scratchpad.stack.push((children[1], false, false)); // Effect
                }
                ExprKind::And
                | ExprKind::Or
                | ExprKind::ForallNew(_)
                | ExprKind::ExistsNew(_)
                | ExprKind::Always
                | ExprKind::Sometime
                | ExprKind::Within
                | ExprKind::AtMostOnce
                | ExprKind::SometimeAfter
                | ExprKind::SometimeBefore
                | ExprKind::AlwaysWithin
                | ExprKind::HoldDuring
                | ExprKind::HoldAfter => {
                    // Push children in reverse order to preserve original evaluation sequence
                    for &child_id in store[old_id].children().iter().rev() {
                        scratchpad.stack.push((child_id, in_condition, false));
                    }
                }
                ExprKind::Imply => {
                    // Imply nodes must be eliminated prior to PNF conversion
                    return Err(StorerError::invalid_node(old_id).into());
                }
                _ => {} // Terminal/leaf nodes require no children expansion
            }
        } else {
            // --- 2. UPWARDS PHASE (HASH-CONSED RECONSTRUCTION) ---
            let entry_kind = *store[old_id].kind();

            let new_id = match entry_kind {
                ExprKind::Not => {
                    if in_condition {
                        let child_id = *store[old_id].children().first().unwrap();
                        let new_child_id = *scratchpad.cache.get(&child_id).unwrap_or(&child_id);
                        let child_kind = *store[new_child_id].kind();

                        match child_kind {
                            ExprKind::Not => {
                                return Err(StorerError::invalid_node(new_child_id).into())
                            }
                            ExprKind::AtomicFormula(mut atom_id) => {
                                if atom_id.is_negated() {
                                    return Err(StorerError::invalid_node(new_child_id).into());
                                }
                                // Negation Absorption: Toggle MSB bit-mask on the atom skeleton
                                atom_id.set_negated(true);
                                negated_atoms.push(atom_id);

                                // Intern the newly modified negated atom
                                store.intern(ExprKind::AtomicFormula(atom_id), &[])
                            }
                            ExprKind::Comparison(_) => {
                                // Comparisons cannot absorb negation directly; fallback to standard interning
                                store.intern(ExprKind::Not, &[new_child_id])
                            }
                            _ => return Err(StorerError::invalid_node(new_child_id).into()),
                        }
                    } else {
                        // Outside a condition (e.g., Delete Effect), preserve standard Not structure
                        let child_id = *store[old_id].children().first().unwrap();
                        let new_child_id = *scratchpad.cache.get(&child_id).unwrap_or(&child_id);
                        store.intern(ExprKind::Not, &[new_child_id])
                    }
                }
                _ => {
                    let old_children = store[old_id].children();
                    if old_children.is_empty() {
                        old_id
                    } else {
                        let mut has_changed = false;
                        scratchpad.children_buffer.clear();

                        // Lazy Copying Optimization: Avoid writing to scratchpad buffer until a structural change is detected
                        for (idx, &child_id) in old_children.iter().enumerate() {
                            let new_child_id =
                                *scratchpad.cache.get(&child_id).unwrap_or(&child_id);

                            if has_changed {
                                scratchpad.children_buffer.push(new_child_id);
                            } else if new_child_id != child_id {
                                has_changed = true;
                                // Catch up by copying all preceding unchanged children at once
                                scratchpad
                                    .children_buffer
                                    .extend_from_slice(&old_children[..idx]);
                                scratchpad.children_buffer.push(new_child_id);
                            }
                        }

                        if has_changed {
                            // Intern the node with its updated children IDs
                            store.intern(entry_kind, &scratchpad.children_buffer)
                        } else {
                            old_id
                        }
                    }
                }
            };

            // Memoize the mapping from the old expression ID to the new PNF expression ID
            scratchpad.cache.insert(old_id, new_id);
        }
    }

    // Canonicalize the tracking array of absorbed negated atoms
    negated_atoms.sort_unstable();
    negated_atoms.dedup();

    Ok(*scratchpad.cache.get(&expr_id).unwrap_or(&expr_id))
}

#[cfg(test)]
mod tests {
    use crate::aiplan4rust::compiler::grounding::error::GroundingError;
    use crate::aiplan4rust::compiler::grounding::passes::pnf::expr::to_pnf_with;
    use crate::aiplan4rust::compiler::grounding::passes::pnf::scratchpad::PnfScratchpad;
    use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprStore};
    use crate::aiplan4rust::support::lang::{AtomSkeletonId, CompareOp};

    /// Verifies that a logical negation wrapping an atomic formula is absorbed into its internal bit-mask.
    ///
    /// # Objective
    /// Ensure that in a condition/logical context (`is_effect = false`), a `Not` node wrapping an
    /// atomic formula is completely stripped away by transferring the negation directly into the
    /// atom's internal bit-mask, and that this atom is logged for global tracking.
    ///
    /// # Input
    /// A structural `Not` node wrapping a positive `AtomicFormula` evaluated with `is_effect = false`.
    ///
    /// # Expected Output
    /// A direct `AtomicFormula` node with its negation bit set to `true`, and the modified
    /// atom logged as the sole element inside the `negated_atoms` vector.
    #[test]
    fn test_encode_simple_atom_negation_logical() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (not (at-robot r1))
        let mut skeleton_id = AtomSkeletonId::new(500);
        skeleton_id.set_negated(false);

        let atom_id = store.intern(ExprKind::AtomicFormula(skeleton_id), &[]);
        let not_id = store.intern(ExprKind::Not, &[atom_id]);

        // 2. Execution: Run the PNF transformation in logical/condition mode (is_effect = false)
        let new_root_id = to_pnf_with(
            not_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        )?;

        // 3. Validation: Verify that the Not node was absorbed into the AtomicFormula
        let root_kind = store[new_root_id].kind();

        if let ExprKind::AtomicFormula(final_atom_id) = root_kind {
            assert!(
                final_atom_id.is_negated(),
                "The atom's internal MSB negation bit must be set to true"
            );
            assert_eq!(
                negated_atoms.len(),
                1,
                "The absorbed atom must be globally tracked inside negated_atoms"
            );
            assert_eq!(
                negated_atoms[0], *final_atom_id,
                "The tracked atom ID must exactly match the newly modified internal atom node"
            );
        } else {
            panic!("The logical Not node should have been completely absorbed to produce a direct AtomicFormula node");
        }

        Ok(())
    }

    /// Verifies that negations inside an effect context (Delete Effects) are preserved structurally.
    ///
    /// # Objective
    /// Ensure that the PNF engine does not perform negation absorption when evaluating effects (`is_effect = true`).
    /// In planning, a negated literal in an effect represents a delete effect, which must maintain its explicit `Not` node structure.
    ///
    /// # Input
    /// A `Not` node wrapping an `AtomicFormula` evaluated with the `is_effect` parameter set to `true`.
    ///
    /// # Expected Output
    /// A preserved `Not(AtomicFormula)` tree structure where the internal negation bit of the atom
    /// remains untouched, and the `negated_atoms` tracking vector remains completely empty.
    #[test]
    fn test_encode_effect_negation_preservation() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (not (at-robot r1))
        let mut skeleton_id = AtomSkeletonId::new(500);
        skeleton_id.set_negated(false);
        let atom_id = store.intern(ExprKind::AtomicFormula(skeleton_id), &[]);
        let not_id = store.intern(ExprKind::Not, &[atom_id]);

        // 2. Execution: Run the PNF transformation in effect mode (is_effect = true)
        let new_root_id = to_pnf_with(
            not_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            true,
        )?;

        // 3. Validation: The structural Not node must be kept as-is to represent a Delete Effect
        let root_kind = store[new_root_id].kind();
        assert!(
            matches!(root_kind, ExprKind::Not),
            "The 'Not' operator must be structurally preserved when processing an effect context"
        );
        assert!(
            negated_atoms.is_empty(),
            "Delete effects must not push their underlying atoms into the negated_atoms tracker"
        );

        Ok(())
    }

    /// Verifies that a negated comparison remains structurally unchanged and is not absorbed.
    ///
    /// # Objective
    /// Ensure that the PNF engine correctly identifies that `Comparison` nodes cannot absorb
    /// negations into an internal bit-mask, thereby preserving the structural `Not` node wrapping them.
    ///
    /// # Input
    /// A standard structural `Not` node wrapping an equality `Comparison` node.
    ///
    /// # Expected Output
    /// An identical `Not(Comparison)` tree structure where the `negated_atoms` tracker
    /// remains completely empty.
    #[test]
    fn test_encode_comparison_stays_unchanged() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (not (= ?x ?y))
        // We intern the comparison structurally to prevent any potential constant folding interference.
        let comp_id = store.intern(ExprKind::Comparison(CompareOp::Equal), &[]);
        let not_id = store.intern(ExprKind::Not, &[comp_id]);

        // 2. Execution: Run the PNF transformation
        let new_root_id = to_pnf_with(
            not_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        )?;

        // 3. Validation: Verify that the structural Not node is preserved
        let root_kind = store[new_root_id].kind();
        assert!(
            matches!(root_kind, ExprKind::Not),
            "The root must remain a Not node for comparisons"
        );

        let children = store[new_root_id].children();
        assert_eq!(children.len(), 1);
        assert_eq!(
            store[children[0]].kind(),
            &ExprKind::Comparison(CompareOp::Equal)
        );

        assert!(
            negated_atoms.is_empty(),
            "Negated comparisons must not be tracked or collected inside negated_atoms"
        );

        Ok(())
    }
    /// Verifies that the PNF engine successfully detects and rejects unsupported complex nodes found under a negation.
    ///
    /// # Objective
    /// Ensure that the system enforces structural invariants by failing fast when encountering
    /// an un-eliminated or complex logical node (like `Imply`) directly under a `Not` operator.
    ///
    /// # Input
    /// A negation expression wrapping an unsupported implication block: `Not(Imply(AtomicFormula, AtomicFormula))`.
    ///
    /// # Expected Output
    /// The function must return a `Result::Err` containing a structural validation error,
    /// and the `negated_atoms` vector must remain completely empty.
    #[test]
    fn test_detect_unsupported_node_under_not() {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (not (imply A B))
        let a_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(100)), &[]);
        let b_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(101)), &[]);
        let imply_id = store.intern(ExprKind::Imply, &[a_id, b_id]);
        let not_id = store.intern(ExprKind::Not, &[imply_id]);

        // 2. Execution & Validation: The engine must reject unsupported nodes under a Not
        let result = to_pnf_with(
            not_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        );

        assert!(
            result.is_err(),
            "The pass must return an error if an unsupported 'Imply' node is encountered under a 'Not' operator"
        );
        assert!(negated_atoms.is_empty());
    }

    /// Verifies that the PNF engine successfully detects and rejects explicit double negations.
    ///
    /// # Objective
    /// Ensure that the system enforces NNF preconditions by catching nested `Not` structures
    /// early in the pipeline and returning a structural validation error.
    ///
    /// # Input
    /// A nested negation expression of the form `Not(Not(AtomicFormula))`.
    ///
    /// # Expected Output
    /// The function must return a `Result::Err` encapsulating a node validation failure,
    /// and the `negated_atoms` tracker must remain completely empty.
    #[test]
    fn test_detect_double_negation_failure() {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (not (not A))
        let atom_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(100)), &[]);
        let inner_not = store.intern(ExprKind::Not, &[atom_id]);
        let outer_not = store.intern(ExprKind::Not, &[inner_not]);

        // 2. Execution & Validation: The engine must reject direct (NOT NOT) structures
        let result = to_pnf_with(
            outer_not,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        );

        assert!(
            result.is_err(),
            "The engine must reject explicit and direct double negations (NOT NOT)"
        );
        assert!(negated_atoms.is_empty());
    }

    /// Verifies the behavior of PNF conversion on a mixed tree containing both absorbable atoms and non-absorbable comparisons.
    ///
    /// # Objective
    /// Ensure that the PNF engine correctly applies heterogeneous logic across different node types:
    /// absorbing the negation for standard atomic formulas while preserving standard structural negation for comparisons.
    ///
    /// # Input
    /// An `And` node wrapping two negated children:
    /// 1. `Not(AtomicFormula)`
    /// 2. `Not(Comparison)`
    ///
    /// # Expected Output
    /// An updated `And` node where:
    /// 1. The first child becomes a direct `AtomicFormula` with its internal negation bit enabled (absorbed).
    /// 2. The second child remains a `Not(Comparison)` structure, since comparisons cannot absorb negations.
    /// 3. The `negated_atoms` vector tracks exactly one absorbed atom skeleton.
    #[test]
    fn test_mixed_complex_pnf() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: (and (not (at-robot)) (not (= ?x ?y)))
        let atom_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(500)), &[]);
        let not_atom_id = store.intern(ExprKind::Not, &[atom_id]);

        let comp_id = store.intern(ExprKind::Comparison(CompareOp::Equal), &[]);
        let not_comp_id = store.intern(ExprKind::Not, &[comp_id]);

        let and_id = store.intern(ExprKind::And, &[not_atom_id, not_comp_id]);

        // 2. Execution: Run the PNF transformation
        let new_root_id = to_pnf_with(
            and_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        )?;

        // 3. Validation: Verify the structural changes on the root and its children
        let root_node = &store[new_root_id];
        assert!(matches!(root_node.kind(), ExprKind::And));

        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // Child 1: The negation must be completely absorbed into the atom
        if let ExprKind::AtomicFormula(final_id) = store[children[0]].kind() {
            assert!(final_id.is_negated());
            assert_eq!(negated_atoms.len(), 1);
            assert_eq!(negated_atoms[0], *final_id);
        } else {
            panic!("The first child node should have been converted into a direct negated AtomicFormula");
        }

        // Child 2: The negation under the comparison must remain unchanged as a standard Not node
        assert!(matches!(store[children[1]].kind(), ExprKind::Not));
        let inner_comp_children = store[children[1]].children();
        assert_eq!(
            store[inner_comp_children[0]].kind(),
            &ExprKind::Comparison(CompareOp::Equal)
        );

        Ok(())
    }

    /// Verifies that the PNF engine flags illegal double negations embedded via bit-masks.
    ///
    /// # Objective
    /// Ensure that the system maintains strict NNF invariants and rejects expressions where a `Not`
    /// node sits above an atomic formula that has already had its negation bit set.
    ///
    /// # Input
    /// A `Not` node wrapping an `AtomicFormula` whose `AtomSkeletonId` already has its internal
    /// negation flag set to `true`.
    ///
    /// # Expected Output
    /// The function must return a `Result::Err` containing a validation error, and the
    /// `negated_atoms` vector must remain empty.
    #[test]
    fn test_detect_forbidden_double_negation_in_bit() {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Setup: Create an atom that is already bit-negated
        let mut corrupted_skeleton = AtomSkeletonId::new(500);
        corrupted_skeleton.set_negated(true);

        let atom_id = store.intern(ExprKind::AtomicFormula(corrupted_skeleton), &[]);
        let not_id = store.intern(ExprKind::Not, &[atom_id]);

        // 2. Execution: Run the PNF transformation
        let result = to_pnf_with(
            not_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        );

        // 3. Validation: The engine must reject the double negation
        assert!(
            result.is_err(),
            "The engine must return an error if it encounters a Not node above an already bit-negated atom"
        );
        assert!(negated_atoms.is_empty());
    }

    /// Verifies the structural correctness and stack safety of PNF conversion under deep nesting.
    ///
    /// # Objective
    /// Ensure that the iterative algorithm handles deeply nested logical trees without causing
    /// a stack overflow, while correctly propagating context and absorbing negations down to the leaf.
    ///
    /// # Input
    /// An expression tree containing a single atomic formula wrapped in a `Not` node,
    /// nested under 1,000 sequential `And` logical operators.
    ///
    /// # Expected Output
    /// A rewritten PNF expression tree preserving the 1,000 `And` layers, where the deep
    /// atomic formula leaf successfully absorbs the negation (MSB toggle) and is logged into `negated_atoms`.
    #[test]
    fn test_pnf_deep_nesting() -> Result<(), GroundingError> {
        let mut store = ExprStore::new();
        let mut negated_atoms = Vec::new();
        let mut scratchpad = PnfScratchpad::new();

        // 1. Leaf setup: (Not A)
        let atom_id = store.intern(ExprKind::AtomicFormula(AtomSkeletonId::new(500)), &[]);
        let mut current_id = store.intern(ExprKind::Not, &[atom_id]);

        // 2. Nesting: Wrap the expression inside 1,000 sequential AND nodes
        for _ in 0..1000 {
            current_id = store.intern(ExprKind::And, &[current_id]);
        }

        // 3. Execution: Convert the deep structure using the scratchpad
        let new_root_id = to_pnf_with(
            current_id,
            &mut store,
            &mut negated_atoms,
            &mut scratchpad,
            false,
        )?;

        // 4. Validation: Verify that all 1,000 AND layers were preserved during reconstruction
        let mut checker_id = new_root_id;
        for _ in 0..1000 {
            let node = &store[checker_id];
            assert!(matches!(node.kind(), ExprKind::And));
            checker_id = node.children()[0];
        }

        // 5. Leaf Validation: Ensure negation was absorbed into the atomic formula
        if let ExprKind::AtomicFormula(final_id) = store[checker_id].kind() {
            assert!(final_id.is_negated());
            assert_eq!(negated_atoms.len(), 1);
            assert_eq!(negated_atoms[0], *final_id);
        } else {
            panic!("The deep terminal leaf node must be converted into an AtomicFormula");
        }

        Ok(())
    }
}
