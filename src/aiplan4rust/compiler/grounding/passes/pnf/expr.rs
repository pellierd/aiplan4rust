use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::pnf::scratchpad::PnfScratchpad;
use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::lir::expr::{validation, Expr, ExprId, ExprKind, ExprStore};
use crate::aiplan4rust::compiler::lir::renderers::{LiftedDebugDisplay, LirRenderContext};
use crate::aiplan4rust::support::lang::AtomSkeletonId;

/// Final lowering of an expression tree into its encoded NNF (Negation Normal Form) via Negation Absorption.
///
/// Convenient entry point that allocates a temporary, high-performance dual-cached scratchpad on the fly.
/// For loops or intensive grounding paths, prefer using [`to_pnf_with`] directly with a shared scratchpad
/// to eliminate dynamic allocations completely.
///
/// # Preconditions
///
/// This function assumes that the expression tree adheres to the following structural constraints:
///
/// 1. **Well-Formed Tree Structure**: Complex logical operators like `Imply` must be eliminated prior to this pass.
/// 2. **Negation Constraints**: Negations (`Not`) must strictly target literal leaves (atoms or comparisons) within
///    condition contexts. Nested structures like `Not(And(...))` or double negations will result in a [`GroundingError`].
///
/// If these structural invariants are violated, the function will immediately return a [`GroundingError`].
pub fn to_pnf(
    expr_id: ExprId,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    is_effect: bool,
) -> Result<ExprId, GroundingError> {
    // SAFETY: If the starting ID is invalid or marked as "None"
    if expr_id.is_none() {
        return Ok(expr_id);
    }
    // Local scratchpad allocation for one-off conversions
    let mut scratchpad = PnfScratchpad::new();
    to_pnf_with(expr_id, store, negated_atoms, &mut scratchpad, is_effect)
}

/// Final lowering of an expression tree into its encoded NNF (Negation Normal Form) via Negation Absorption.
///
/// Reconstructs the tree bottom-up using hash-consing and returns the new [`ExprId`].
///
/// # Preconditions
///
/// The NNF conversion and absorption algorithm relies on strict structural invariants:
///
/// * **QNF / Quantifier Normal Form (Optional/Flexible)**: If quantifiers (`ForallNew`, `ExistsNew`)
///   are present, they are safely traversed, and their child scopes inherit the correct logical context.
///   However, complex logical connectives like `Imply` must be entirely eliminated prior to this pass.
/// * **Well-Formed NNF Structure**: Complex nested negations directly targeting non-literal blocks
///   (e.g., `Not(And(...))` or double negations `Not(Not(...))`) within a condition context are forbidden
///   and will immediately return a `StorerError::invalid_node` error.
///
/// # Layout & Performance Optimizations
///
/// * **Dual Flat Lookup Tables**: Replaces traditional `HashMap` caches with dual pre-allocated `Vec` fields
///   (`cache_true` and `cache_false`) within the scratchpad. Since `ExprId` matches sequential arena indices,
///   cache lookups and mutations achieve a raw temporal complexity of **O(1)** with maximum hardware prefetching locality.
/// * **Zero-Allocation Critical Path**: The scratchpad caches are resized and reset sequentially using SIMD-vectorized
///   `.fill()` operations synchronized with the current `ExprStore::len()`, avoiding heap fragmentation during grounding loops.
/// * **Lazy Copying**: Avoids writing to the scratchpad's child accumulation buffers until a structural change
///   (negation absorption) is explicitly detected, allowing unchanged sub-trees to be mirrored in $O(1)$.
///
/// # Algorithm
///
/// The process is strictly iterative (backed by an explicit stack) to prevent stack overflows on deeply nested
/// PDDL/HDDL syntax trees. It operates in a two-phase sequence:
///
/// 1. **Downwards Phase**: Propagates the logical evaluation context (`in_condition`, distinguishing between
///    preconditions and effects/delete-effects) and performs fail-fast cache hits.
/// 2. **Upwards Phase**: Absorbs valid negations directly into the atom's bit-mask (`set_negated(true)`)
///    and reconstructs the updated tree nodes via `ExprStore` interning.
pub fn to_pnf_with(
    expr_id: ExprId,
    store: &mut ExprStore,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut PnfScratchpad,
    is_effect: bool,
) -> Result<ExprId, GroundingError> {
    // SAFETY: If the starting ID is invalid or marked as "None"
    if expr_id.is_none() {
        return Ok(expr_id);
    }

    // Cache resize optimization: Resize the scratchpad caches to match the current ExprStore length
    scratchpad.clear(store.len());

    // Sentinel de détection (équivalent à ExprId::NONE ou ton ExprId par défaut)
    let default_id = ExprId::default();

    // Bootstrap the stack: (ExprId, InCondition, ChildrenPushed)
    scratchpad.stack.push((expr_id, !is_effect, false));

    while let Some((old_id, in_condition, children_pushed)) = scratchpad.stack.pop() {
        let old_idx = old_id.as_usize();

        if !children_pushed {
            // --- 1. DOWNWARDS PHASE (CONTEXT PROPAGATION & VALIDATION) ---

            // Extraction directe O(1) de la bonne table de cache selon le contexte descendant
            let cache = if in_condition {
                &scratchpad.cache_true
            } else {
                &scratchpad.cache_false
            };
            if cache[old_idx] != default_id {
                continue;
            }

            // Re-push current node with children_pushed=true to process it during the upwards phase
            scratchpad.stack.push((old_id, in_condition, true));

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
                | ExprKind::Forall(_)
                | ExprKind::Exists(_)
                | ExprKind::AtStart
                | ExprKind::AtEnd
                | ExprKind::Overall
                | ExprKind::Always
                | ExprKind::Sometime
                | ExprKind::Within
                | ExprKind::AtMostOnce
                | ExprKind::SometimeAfter
                | ExprKind::SometimeBefore
                | ExprKind::AlwaysWithin
                | ExprKind::HoldDuring
                | ExprKind::HoldAfter
                | ExprKind::Preference => {
                    // Push children in reverse order to preserve original evaluation sequence
                    for &child_id in store[old_id].children().iter().rev() {
                        scratchpad.stack.push((child_id, in_condition, false));
                    }
                }
                ExprKind::Imply => {
                    return Err(StorerError::invalid_node(old_id).into());
                }
                _ => {} // Terminal/leaf nodes require no children expansion
            }
        } else {
            // --- 2. UPWARDS PHASE (HASH-CONSED RECONSTRUCTION) ---

            let cache = if in_condition {
                &scratchpad.cache_true
            } else {
                &scratchpad.cache_false
            };
            if cache[old_idx] != default_id {
                continue;
            }

            let entry_kind = *store[old_id].kind();

            let new_id = match entry_kind {
                ExprKind::Not => {
                    if in_condition {
                        let child_id = *store[old_id].children().first().unwrap();

                        // Lecture O(1) de la version transformée du fils
                        let cached_child = scratchpad.cache_true[child_id.as_usize()];
                        let new_child_id = if cached_child != default_id {
                            cached_child
                        } else {
                            child_id
                        };

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
                            ExprKind::Comparison(_) => store.intern(ExprKind::Not, &[new_child_id]),
                            _ => {
                                // 🎯 On print le type de nœud inattendu pour le démasquer dans la console
                                println!(
                                    "💥 PNF ERROR: Unexpected child_kind `{:?}` inside Not (id: {})",
                                    child_kind,
                                    new_child_id.as_usize()
                                );
                                return Err(StorerError::invalid_node(new_child_id).into());
                            }
                        }
                    } else {
                        // Outside a condition (Delete Effect), preserve standard Not structure
                        let child_id = *store[old_id].children().first().unwrap();
                        let cached_child = scratchpad.cache_false[child_id.as_usize()];
                        let new_child_id = if cached_child != default_id {
                            cached_child
                        } else {
                            child_id
                        };

                        store.intern(ExprKind::Not, &[new_child_id])
                    }
                }
                ExprKind::When => {
                    let old_children = store[old_id].children();
                    let cond_child = old_children[0];
                    let eff_child = old_children[1];

                    // La condition doit TOUJOURS lire cache_true
                    let cached_cond = scratchpad.cache_true[cond_child.as_usize()];
                    let new_cond_id = if cached_cond != default_id {
                        cached_cond
                    } else {
                        cond_child
                    };

                    // L'effet doit TOUJOURS lire cache_false
                    let cached_eff = scratchpad.cache_false[eff_child.as_usize()];
                    let new_eff_id = if cached_eff != default_id {
                        cached_eff
                    } else {
                        eff_child
                    };

                    if new_cond_id != cond_child || new_eff_id != eff_child {
                        store.intern(ExprKind::When, &[new_cond_id, new_eff_id])
                    } else {
                        old_id
                    }
                }
                _ => {
                    let old_children = store[old_id].children();
                    if old_children.is_empty() {
                        old_id
                    } else {
                        let mut has_changed = false;
                        scratchpad.children_buffer.clear();

                        // Lazy Copying Optimization via flat lookup table mapping
                        let active_cache = if in_condition {
                            &scratchpad.cache_true
                        } else {
                            &scratchpad.cache_false
                        };

                        for (idx, &child_id) in old_children.iter().enumerate() {
                            let cached_child = active_cache[child_id.as_usize()];
                            let new_child_id = if cached_child != default_id {
                                cached_child
                            } else {
                                child_id
                            };

                            if has_changed {
                                scratchpad.children_buffer.push(new_child_id);
                            } else if new_child_id != child_id {
                                has_changed = true;
                                scratchpad
                                    .children_buffer
                                    .extend_from_slice(&old_children[..idx]);
                                scratchpad.children_buffer.push(new_child_id);
                            }
                        }

                        if has_changed {
                            store.intern(entry_kind, &scratchpad.children_buffer)
                        } else {
                            old_id
                        }
                    }
                }
            };

            // Écriture directe O(1) dans la table mutable appropriée
            let cache_mut = if in_condition {
                &mut scratchpad.cache_true
            } else {
                &mut scratchpad.cache_false
            };
            cache_mut[old_idx] = new_id;
        }
    }

    // Canonicalize the tracking array of absorbed negated atoms
    negated_atoms.sort_unstable();
    negated_atoms.dedup();

    // Résolution finale de la racine
    let final_cache = if !is_effect {
        &scratchpad.cache_true
    } else {
        &scratchpad.cache_false
    };
    let final_root = final_cache[expr_id.as_usize()];

    let final_root = if final_root != default_id {
        final_root
    } else {
        expr_id
    };

    // Localized post-condition safety check
    check_post_condition(store, final_root, is_effect);

    Ok(final_root)
}

/// Asserts and validates the structural post-conditions of the Precondition Normal Form (PNF) conversion.
///
/// This utility guarantees that the generated expression tree does not contain any logical violations
/// post-transformation. If a violation is detected while `debug_assertions` are enabled, it safe-renders
/// the structural tree to the standard output before panicking via a `debug_assert!`.
///
/// # Invariants Checked
///
/// * In a **pure condition context** (`is_effect = false`), all negations (`Not`) must be absorbed into
///   literals, with the sole exception of negations directly wrapping a `Comparison` node.
/// * In an **effect context** (`is_effect = true`), top-level literal negations are permitted, while any
///   nested condition blocks (e.g., within conditional `When` effects) are audited under strict condition rules.
///
/// # Arguments
///
/// * `store` - A reference to the [`ExprStore`] hosting the expression nodes.
/// * `root_id` - The [`ExprId`] representing the root of the newly transformed subtree.
/// * `is_effect` - A boolean flag specifying whether the verification context is an action effect (`true`) or a condition block (`false`).
///
/// # Performance
///
/// This function is marked with `#[inline(always)]`. In production compilation builds (`--release` / without `debug_assertions`),
/// the function body is completely optimized out by the compiler, incurring **zero runtime overhead**.
#[inline(always)]
fn check_post_condition(store: &ExprStore, root_id: ExprId, is_effect: bool) {
    if cfg!(debug_assertions) {
        // validation::is_pnf consumes the store, the root ID, and the contextual boolean
        if !validation::is_pnf(store, root_id, is_effect) {
            println!("\n=== [DEBUG] CRASH DETECTED IN PNF CONVERSION ===");
            println!("Root ExprId: {:?}", root_id);
            println!("Is Effect Context: {}", is_effect);

            // Render the unified structural tree via the debug RenderContext
            let ctx = LirRenderContext::debug(store);
            let expr_handle = Expr::new(root_id, store);
            println!("{}", expr_handle.as_debug(&ctx));

            println!("================================================\n");
        }
    }

    debug_assert!(
        validation::is_pnf(store, root_id, is_effect),
        "LOGICAL VIOLATION: PNF conversion failed! Unabsorbed 'Not' operators were found inside a pure condition context branch."
    );
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
