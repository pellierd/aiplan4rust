use crate::aiplan4rust::compiler::lir::expr::iter::Scratchpad;
use crate::aiplan4rust::compiler::lir::expr::ops::{fnf, nnf, tnf};
use crate::aiplan4rust::compiler::lir::expr::{is_fnf, is_nnf, ExprBuilder, ExprId, ExprStore};
use crate::aiplan4rust::compiler::lir::normalization::error::NormalizationError;

/// Independent normalization pipeline.
///
/// Takes a root expression ID and orchestrates sequential transformations
/// (NNF, TNF, FNF) without being bound to a specific `Expr` or `LiftedProblem` instance.
///
/// # Parameters
///
/// * `root` - The unique identifier of the root expression node in the expression store.
/// * `store` - A mutable reference to the central storage containing all expression nodes.
/// * `scratch` - A mutable reference to a reusable allocation buffer to optimize tree traversals.
/// * `is_durative` - A flag indicating whether the expression belongs to a durative (temporal) context.
/// * `is_effect` - A flag specifying whether the expression is processed as an action effect (`true`) or a condition (`false`).
///
/// # Returns
///
/// * `Ok(ExprId)` - The `ExprId` of the newly generated, fully optimized root node.
/// * `Err(NormalizationError)` - An error variant reflecting a failure during pipeline phases.
pub fn normalize(
    root: ExprId,
    store: &mut ExprStore,
    scratch: &mut Scratchpad,
    is_durative: bool,
    is_effect: bool,
) -> Result<ExprId, NormalizationError> {
    let mut builder = ExprBuilder::new(store);

    // 1. Negation Normal Form (NNF)
    // Pushes negations inwards down to the literal level.
    let root = nnf::to_nnf(root, &mut builder, scratch)?;

    // 2. Temporal Normal Form (TNF)
    // Decomposes temporal structures into synchronized streams (start, end, overall).
    let root = if is_durative {
        tnf::to_tnf(root, &mut builder, scratch, is_effect)?
    } else {
        root
    };

    // 3. Flattening and Factorization Normal Form (FNF)
    // Flattens associative operators (AND/OR) and eliminates structural redundancies.
    let final_root = fnf::to_fnf(root, &mut builder, scratch, true)?;

    // Global pipeline validation
    check_post_conditions(builder.store(), root, final_root);

    Ok(final_root)
}

/// Validates global pipeline invariants and cross-phase post-conditions in Debug mode.
/// Has zero runtime overhead in Release profiles.
///
/// # Parameters
///
/// * `store` - A reference to the central expression storage.
/// * `pre_fnf_root` - The `ExprId` of the tree right before entering the FNF phase.
/// * `final_root` - The final processed `ExprId` after all normalization steps.
#[inline]
fn check_post_conditions(store: &ExprStore, pre_fnf_root: ExprId, final_root: ExprId) {
    // 1. Ensure downstream phases (TNF/FNF) did not corrupt the Negation Normal Form contract.
    debug_assert!(
        is_nnf(store, final_root),
        "CRITICAL REGRESSION: TNF or FNF corrupted the Negation Normal Form (NNF)!"
    );

    // 2. Ensure the structural integrity and flattening invariants of the final tree are met.
    debug_assert!(
        is_fnf(store, pre_fnf_root, final_root),
        "CRITICAL REGRESSION: Final tree violates Flat Normal Form (FNF)!"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprKind, ExprStore};
    use crate::aiplan4rust::support::lang::{ArithmeticOp, AtomSkeletonId, PredicateSymbolId};

    /// Complex test for AND-flattening and structural deduplication.
    ///
    /// Input: (and (and A B) (and B C) (and (and A B) D))
    /// Expected: (and A B C D)
    #[test]
    fn test_complex_nested_and_deduplication() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atoms: A=1, B=2, C=3, D=4
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let c = builder.atomic_formula(PredicateSymbolId::from(3), &[], skel);
        let d = builder.atomic_formula(PredicateSymbolId::from(4), &[], skel);

        // 2. Build nested structure: (and (and 1 2) (and 2 3) (and (and 1 2) 4))
        let inner1 = builder.and(&[a, b]);
        let inner2 = builder.and(&[b, c]);
        let inner3 = builder.and(&[inner1, d]);
        let root = builder.and(&[inner1, inner2, inner3]);

        // 3. Normalize (is_durative = false, is_effect = false)
        // Non-durative mode ensures the TNF doesn't wrap literals in temporal triplets.
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root must be a logical AND
        assert!(
            matches!(entry.kind(), ExprKind::And),
            "Root should be an AND node, found {:?}",
            entry.kind()
        );

        // The result should have exactly 4 unique children [A, B, C, D]
        // after recursive flattening and hash-consing deduplication.
        assert_eq!(
            entry.children().len(),
            4,
            "Expression should be flattened to 4 unique atoms"
        );

        Ok(())
    }

    /// Tests structural simplification of AND nodes with nested identical sub-trees.
    ///
    /// Input: (and A (and B C) (and B C))
    /// Steps:
    ///   1. Flattening -> (and A B C B C)
    ///   2. Deduplication -> (and A B C)
    /// Expected: (and A B C)
    #[test]
    fn test_root_and_structural_simplification() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atoms: A=1, B=2, C=3
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let c = builder.atomic_formula(PredicateSymbolId::from(3), &[], skel);

        // 2. Construct sub-trees: inner1 and inner2 are structurally identical (and B C)
        // Thanks to Hash-Consing, inner1 and inner2 will already share the same ID.
        let inner1 = builder.and(&[b, c]);
        let inner2 = builder.and(&[b, c]);

        // 3. Construct root: (and A (and B C) (and B C))
        let root = builder.and(&[a, inner1, inner2]);

        // 4. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root must remain an AND node
        assert!(
            matches!(entry.kind(), ExprKind::And),
            "Root should be an AND node"
        );

        // After flattening and deduplication, we expect exactly 3 children: A, B, and C.
        // Flattened: (and 1 2 3 2 3) -> Deduplicated: (and 1 2 3)
        assert_eq!(
            entry.children().len(),
            3,
            "Root should have exactly 3 children after flattening and deduplication"
        );

        // Ensure all resulting children are AtomicFormulas (the leaf atoms)
        for &child_id in entry.children() {
            let child = store.fetch(child_id)?;
            assert!(
                matches!(child.kind(), ExprKind::AtomicFormula(_)),
                "Child {:?} should be an AtomicFormula",
                child_id
            );
        }

        Ok(())
    }

    /// Tests structural simplification of OR nodes with nested identical sub-trees.
    ///
    /// Input: (or A (or B C) (or B C))
    /// Steps:
    ///   1. Flattening -> (or A B C B C)
    ///   2. Deduplication -> (or A B C)
    /// Expected: (or A B C)
    #[test]
    fn test_root_or_structural_simplification() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atoms: A=1, B=2, C=3
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let c = builder.atomic_formula(PredicateSymbolId::from(3), &[], skel);

        // 2. Create two structurally identical sub-trees: (or B C)
        // Hash-consing ensures inner1 and inner2 point to the same ID.
        let inner1 = builder.or(&[b, c]);
        let inner2 = builder.or(&[b, c]);

        // 3. Initial structure: (or A (or B C) (or B C))
        let root = builder.or(&[a, inner1, inner2]);

        // 4. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root must be an OR node
        assert!(
            matches!(entry.kind(), ExprKind::Or),
            "Root should be an OR node"
        );

        // After flattening and deduplication, we expect exactly 3 unique children: A, B, and C.
        assert_eq!(
            entry.children().len(),
            3,
            "Root should have exactly 3 children after flattening and deduplication"
        );

        // Verify that every resulting child is an AtomicFormula
        for &child_id in entry.children() {
            let child = store.fetch(child_id)?;
            assert!(
                matches!(child.kind(), ExprKind::AtomicFormula(_)),
                "Child {:?} should be an AtomicFormula",
                child_id
            );
        }

        Ok(())
    }

    /// Tests structural deduplication in a root OR node with duplicate sub-trees,
    /// verifying that flattening handles different child orders correctly.
    ///
    /// Input: (or (or A B) (or B A) C)
    /// Steps:
    ///   1. Flattening -> (or A B B A C)
    ///   2. Deduplication -> (or A B C)
    /// Expected: (or A B C)
    #[test]
    fn test_root_or_structural_duplicates_order_independent() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atoms: A=1, B=2, C=3
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let c = builder.atomic_formula(PredicateSymbolId::from(3), &[], skel);

        // 2. Construct sub-trees with different child orders:
        // inner1: (or A B)
        // inner2: (or B A)
        let inner1 = builder.or(&[a, b]);
        let inner2 = builder.or(&[b, a]);

        // 3. Root structure: (or (or A B) (or B A) C)
        let root = builder.or(&[inner1, inner2, c]);

        // 4. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        assert!(
            matches!(entry.kind(), ExprKind::Or),
            "Root should be an OR node"
        );

        // After flattening and deduplication, we expect exactly 3 unique children: A, B, and C.
        // The sequence [A, B, B, A, C] must be reduced to 3 IDs.
        assert_eq!(
            entry.children().len(),
            3,
            "Should have exactly 3 children after order-independent deduplication"
        );

        // Verify all children are atoms
        for &child_id in entry.children() {
            let child = store.fetch(child_id)?;
            assert!(
                matches!(child.kind(), ExprKind::AtomicFormula(_)),
                "Child {:?} should be an AtomicFormula",
                child_id
            );
        }

        Ok(())
    }

    /// Tests the reduction of single-child AND chains.
    ///
    /// Input: (and (and A))
    /// Steps:
    ///   1. Flattening/Reduction -> A
    /// Expected: A (AtomicFormula)
    #[test]
    fn test_and_single_child_reduction() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atom A (PredicateSymbolId = 1, Args = &[], SkeletonId = 0)
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);

        // 2. Create a nested structure with single children: (and (and A))
        let inner = builder.and(&[a]);
        let root = builder.and(&[inner]);

        // 3. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root should no longer be an AND node.
        // It should have been reduced directly to the AtomicFormula.
        assert!(
            matches!(entry.kind(), ExprKind::AtomicFormula(_)),
            "The single-child AND chain should be reduced to the leaf atom, found: {:?}",
            entry.kind()
        );

        // On récupère l'ID du premier enfant (le symbole du prédicat)
        let pred_leaf_id = entry
            .children()
            .get(0)
            .copied()
            .expect("AtomicFormula should have at least one child (the predicate leaf)");

        let pred_leaf = store.fetch(pred_leaf_id)?;

        // Correction ici : On extrait le PredicateSymbolId via le bon variant de ton énumération
        if let ExprKind::PredicateSymbol(pid) = pred_leaf.kind() {
            assert_eq!(pid.as_usize(), 1, "The predicate ID must be 1");
        } else {
            panic!(
                "Root's child should be a predicate ID leaf, found: {:?}",
                pred_leaf.kind()
            );
        }

        Ok(())
    }

    /// Tests that an empty AND node is preserved.
    ///
    /// Input: (and)
    /// Expected: (and) [Logical True]
    #[test]
    fn test_empty_and_node() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create an empty AND node: (and)
        let root = builder.and(&[]);

        // 2. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // Ensure it remains an AND node
        assert!(
            matches!(entry.kind(), ExprKind::And),
            "Empty AND should remain an AND node"
        );

        // Ensure it remains empty
        assert_eq!(
            entry.children().len(),
            0,
            "Empty AND node should remain empty (representing True)"
        );

        Ok(())
    }

    /// Tests that an empty OR node is preserved.
    ///
    /// Input: (or)
    /// Expected: (or) [Logical False]
    #[test]
    fn test_empty_or_node() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create an empty OR node: (or)
        let root = builder.or(&[]);

        // 2. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // Ensure it remains an OR node
        assert!(
            matches!(entry.kind(), ExprKind::Or),
            "Empty OR should remain an OR node"
        );

        // Ensure it remains empty
        assert_eq!(
            entry.children().len(),
            0,
            "Empty OR node should remain empty (representing False)"
        );

        Ok(())
    }

    /// Simplify NOT: double negation.
    ///
    /// Input: (not (not A))
    /// Expected: A (AtomicFormula)
    #[test]
    fn test_simplify_node_double_negation() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atom A (PredicateSymbolId = 1, Args = &[], SkeletonId = 0)
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);

        // 2. Creating nested NOTs: (not (not A))
        let inner_not = builder.not(a);
        let root = builder.not(inner_not);

        // 3. Normalize (is_durative = false, is_effect = false)
        // The internal NNF (Negation Normal Form) pass will eliminate the double negation.
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root node should no longer be a NOT, but directly the AtomicFormula
        assert!(
            matches!(entry.kind(), ExprKind::AtomicFormula(_)),
            "Double negation should be eliminated, leaving only the atom, found: {:?}",
            entry.kind()
        );

        // Retrieve the ID of the first child (the predicate symbol)
        let pred_leaf_id = entry
            .children()
            .get(0)
            .copied()
            .expect("AtomicFormula should have at least one child (the predicate leaf)");

        let pred_leaf = store.fetch(pred_leaf_id)?;

        // Verify that it matches our PredicateSymbol with value 1
        if let ExprKind::PredicateSymbol(pid) = pred_leaf.kind() {
            assert_eq!(pid.as_usize(), 1, "The predicate ID must be 1");
        } else {
            panic!(
                "Root's child should be a predicate ID leaf, found: {:?}",
                pred_leaf.kind()
            );
        }

        Ok(())
    }

    /// Simplify NOT over empty AND.
    ///
    /// Input: (not (and)) [not True]
    /// Expected: (or) [False]
    #[test]
    fn test_simplify_node_not_over_empty_and() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create an empty AND (Logical True)
        let empty_and = builder.and(&[]);

        // 2. Create NOT over empty AND: (not (and))
        let root = builder.not(empty_and);

        // 3. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root should now be an OR node (representing False)
        assert!(
            matches!(entry.kind(), ExprKind::Or),
            "Negating an empty AND should result in an empty OR (False), found: {:?}",
            entry.kind()
        );

        // The OR node should be empty
        assert_eq!(
            entry.children().len(),
            0,
            "The resulting OR node should have no children (Logical False)"
        );

        Ok(())
    }

    /// Double negation over AND subtree.
    ///
    /// Input: (not (not (and A B)))
    /// Expected: (and A B)
    #[test]
    fn test_simplify_node_double_negation_on_and() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atoms: A=1, B=2
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let and_ab = builder.and(&[a, b]);

        // 2. Constructing (not (not (and A B)))
        let inner_not = builder.not(and_ab);
        let root = builder.not(inner_not);

        // 3. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root should now be the AND node (both NOT layers stripped)
        assert!(
            matches!(entry.kind(), ExprKind::And),
            "Double negation over AND should result in the AND node being promoted to root, found: {:?}",
            entry.kind()
        );

        // The AND node should still have its 2 original children (A and B)
        assert_eq!(
            entry.children().len(),
            2,
            "The resulting AND node should preserve its 2 children"
        );

        // Verify the children are the expected AtomicFormulas
        for &child_id in entry.children() {
            let child = store.fetch(child_id)?;
            assert!(
                matches!(child.kind(), ExprKind::AtomicFormula(_)),
                "Child {:?} should be an AtomicFormula",
                child_id
            );
        }

        Ok(())
    }

    /// Input: (A -> (B -> C))
    /// Expected output after flattening: (or (not A) (not B) C)
    #[test]
    fn test_nested_imply_left_to_right() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atoms: A=1, B=2, C=3
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let c = builder.atomic_formula(PredicateSymbolId::from(3), &[], skel);

        // 2. Construct nested implication: (imply A (imply B C))
        let inner_imply = builder.imply(b, c);
        let outer_imply = builder.imply(a, inner_imply);

        // 3. Normalize (is_durative = false, is_effect = false)
        // Convert outer: (or (not A) (imply B C))
        // Convert inner: (or (not A) (or (not B) C))
        // Flatten ORs:   (or (not A) (not B) C)
        let root_after = normalize(outer_imply, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root should be an OR node
        assert!(
            matches!(entry.kind(), ExprKind::Or),
            "Root should be an OR node, found: {:?}",
            entry.kind()
        );

        // After flattening, we expect 3 children: (not A), (not B), and C
        assert_eq!(
            entry.children().len(),
            3,
            "Nested implications should be flattened into a single OR with 3 children"
        );

        // Verify the presence of two NOT nodes and one AtomicFormula
        let mut not_count = 0;
        let mut atom_count = 0;

        for &child_id in entry.children() {
            let child = store.fetch(child_id)?;
            match child.kind() {
                ExprKind::Not => not_count += 1,
                ExprKind::AtomicFormula(_) => atom_count += 1,
                _ => panic!(
                    "Unexpected node kind in normalized implication: {:?}",
                    child.kind()
                ),
            }
        }

        assert_eq!(not_count, 2, "Expected two negated antecedents");
        assert_eq!(atom_count, 1, "Expected one positive consequent");

        Ok(())
    }

    /// Input: ((A -> B) -> C)
    /// Expected output (with De Morgan applied): (or (and A (not B)) C)
    #[test]
    fn test_nested_imply_right_to_left() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atoms: A=1, B=2, C=3
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let c = builder.atomic_formula(PredicateSymbolId::from(3), &[], skel);

        // 2. Construct nested implication: (imply (imply A B) C)
        let inner_imply = builder.imply(a, b);
        let outer_imply = builder.imply(inner_imply, c);

        // 3. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(outer_imply, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root should be an OR node
        assert!(
            matches!(entry.kind(), ExprKind::Or),
            "Root should be an OR node, found: {:?}",
            entry.kind()
        );

        // It should have 2 main children: C and the transformed antecedent
        assert_eq!(
            entry.children().len(),
            2,
            "The outer OR should have exactly two children (the consequent and the transformed antecedent)"
        );

        // Count node kinds under the root OR to validate the structure
        let mut has_and = false;
        let mut has_atom = false;

        for &child_id in entry.children() {
            let child = store.fetch(child_id)?;
            match child.kind() {
                ExprKind::And => has_and = true,
                ExprKind::AtomicFormula(_) => has_atom = true,
                // In case NNF does not push De Morgan down and keeps NOT on the surface:
                ExprKind::Not => has_and = true,
                _ => {}
            }
        }

        assert!(
            has_and,
            "The antecedent must be transformed into a sub-tree (And or Not) representing the negation of (A -> B)"
        );
        assert!(
            has_atom,
            "The consequent (C) must be one of the children of the root OR"
        );

        Ok(())
    }

    /// Nested addition and multiplication:
    ///
    /// Input: (+ 1 (* 2 3) 4)
    /// Expected: 11
    #[test]
    fn test_add_mul_nested() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Numeric constants
        let one = builder.number(1.0);
        let two = builder.number(2.0);
        let three = builder.number(3.0);
        let four = builder.number(4.0);

        // 2. Constructing the expression: (+ 1 (* 2 3) 4)
        let mul = builder.mul(&[two, three]);
        let root = builder.add(&[one, mul, four]);

        // 3. Normalize (is_durative = false, is_effect = false)
        // 1. Evaluate (* 2 3) -> 6.0
        // 2. Evaluate (+ 1 6 4) -> 11.0
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root should now be a single Number node (constant folding)
        assert!(
            matches!(entry.kind(), ExprKind::Number(_)),
            "Nested arithmetic constants should be folded into a single Number, found: {:?}",
            entry.kind()
        );

        // Verify the value is exactly 11.0
        if let ExprKind::Number(val) = entry.kind() {
            // Note: Adapt '.into_inner()' or '.as_f64()' depending on your Float/OrderedFloat type methods
            assert_eq!(
                val.into_inner(),
                11.0,
                "The result of (+ 1 (* 2 3) 4) should be 11.0"
            );
        } else {
            panic!("Root content should be a number variant");
        }

        Ok(())
    }

    /// Nested division and subtraction:
    ///
    /// Input: (- (/ 20 2) 3)
    /// Expected: 7
    #[test]
    fn test_div_sub_nested() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Numeric constants
        let twenty = builder.number(20.0);
        let two = builder.number(2.0);
        let three = builder.number(3.0);

        // 2. Constructing the expression: (- (/ 20 2) 3)
        let div = builder.div(&[twenty, two]);
        let root = builder.sub(&[div, three]);

        // 3. Normalize (is_durative = false, is_effect = false)
        // 1. Evaluate (/ 20 2) -> 10.0
        // 2. Evaluate (- 10 3) -> 7.0
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // Check that the tree collapsed into a single Number node
        assert!(
            matches!(entry.kind(), ExprKind::Number(_)),
            "Nested arithmetic constants should be folded into a single Number, found: {:?}",
            entry.kind()
        );

        // Verify the constant value
        if let ExprKind::Number(val) = entry.kind() {
            assert_eq!(
                val.into_inner(),
                7.0,
                "The result of (- (/ 20 2) 3) should be 7.0"
            );
        } else {
            panic!("Root content should be a number variant");
        }

        Ok(())
    }

    /// Deeply nested operations: (+ (* 2 3) (- 10 4) (/ 20 5))
    ///
    /// Input: (+ (* 2 3) (- 10 4) (/ 20 5))
    /// Expected: 6 + 6 + 4 = 16
    #[test]
    fn test_deeply_nested_operations() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Numeric constants
        let two = builder.number(2.0);
        let three = builder.number(3.0);
        let ten = builder.number(10.0);
        let four = builder.number(4.0);
        let twenty = builder.number(20.0);
        let five = builder.number(5.0);

        // Branch 1: (* 2 3) = 6
        let mul = builder.mul(&[two, three]);
        // Branch 2: (- 10 4) = 6
        let sub = builder.sub(&[ten, four]);
        // Branch 3: (/ 20 5) = 4
        let div = builder.div(&[twenty, five]);

        // Root: (+ 6 6 4) = 16
        let root = builder.add(&[mul, sub, div]);

        // 3. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // Ensure the entire tree collapsed into a single Number
        assert!(
            matches!(entry.kind(), ExprKind::Number(_)),
            "Nested operations should be fully folded, found: {:?}",
            entry.kind()
        );

        // Verify the final calculated value
        if let ExprKind::Number(val) = entry.kind() {
            assert_eq!(
                val.into_inner(),
                16.0,
                "The expression (+ (* 2 3) (- 10 4) (/ 20 5)) should fold to 16.0"
            );
        } else {
            panic!("Root content should be a number variant");
        }

        Ok(())
    }

    /// Nested operation with non-constant child should remain unchanged:
    ///
    /// Input: (+ 2 (* A 3))
    /// Expected: (+ 2 (* A 3)) (cannot simplify because A is a variable)
    #[test]
    fn test_nested_with_variable_child() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Numeric constants & Variable
        let two = builder.number(2.0);
        let three = builder.number(3.0);

        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);

        // 2. Constructing the expression: (+ 2 (* A 3))
        let mul = builder.mul(&[a, three]);
        let root = builder.add(&[two, mul]);

        // 3. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(root, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // 1. Verify the root is still an Addition operation
        assert!(
            matches!(entry.kind(), ExprKind::Arithmetic(ArithmeticOp::Add)),
            "Root should still be an Add node, found: {:?}",
            entry.kind()
        );

        // 2. Verify structure: Addition should have exactly 2 children
        let children = entry.children();
        assert_eq!(children.len(), 2, "Addition should still have 2 children");

        // Find the multiplication child inside the root addition node
        let mut mul_child_entry = None;
        for &child_id in children {
            let child = store.fetch(child_id)?;
            if matches!(child.kind(), ExprKind::Arithmetic(ArithmeticOp::Mul)) {
                mul_child_entry = Some(child);
                break;
            }
        }

        let mul_node =
            mul_child_entry.expect("Multiplication node should still exist under the addition");

        // 3. Verify the multiplication still contains 'A' and '3.0'
        let mul_children = mul_node.children();
        assert_eq!(
            mul_children.len(),
            2,
            "Multiplication should still have 2 children"
        );

        let mut found_a = false;
        let mut found_three = false;

        for &c_id in mul_children {
            let c_node = store.fetch(c_id)?;
            match c_node.kind() {
                ExprKind::AtomicFormula(_) => found_a = true,
                ExprKind::Number(val) => {
                    if val.into_inner() == 3.0 {
                        found_three = true;
                    }
                }
                _ => (),
            }
        }

        assert!(found_a, "Variable 'A' not found in multiplication");
        assert!(found_three, "Constant '3.0' not found in multiplication");

        Ok(())
    }

    /// Simple implication transformation:
    ///
    /// Input: (imply A B)
    /// Expected: (or (not A) B)
    #[test]
    fn test_normalize_simple_imply() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atoms: A=1, B=2
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);

        // 2. Creating the implication: (imply A B)
        let imply = builder.imply(a, b);

        // 3. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(imply, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root should now be an OR node
        assert!(
            matches!(entry.kind(), ExprKind::Or),
            "Implication should be transformed into an OR node, found: {:?}",
            entry.kind()
        );

        // It should have exactly two children: (not A) and B
        assert_eq!(
            entry.children().len(),
            2,
            "The resulting OR node should have 2 children"
        );

        // Verify the structure of the children.
        let mut has_not = false;
        let mut has_atom = false;

        for &child_id in entry.children() {
            let child = store.fetch(child_id)?;
            match child.kind() {
                ExprKind::Not => has_not = true,
                ExprKind::AtomicFormula(_) => has_atom = true,
                _ => {}
            }
        }

        assert!(has_not, "Missing negated antecedent (not A)");
        assert!(has_atom, "Missing consequent (B)");

        Ok(())
    }

    /// Combination of implication and double negation:
    ///
    /// Input: (imply (not (not A)) B)
    /// Process: (or (not (not (not A))) B) -> (or (not A) B)
    /// Expected: (or (not A) B)
    #[test]
    fn test_normalize_with_double_negation() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atoms: A=1, B=2
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let not_a = builder.not(a);
        let double_not_a = builder.not(not_a);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);

        // 2. Construction: (imply (not (not A)) B)
        let imply = builder.imply(double_not_a, b);

        // 3. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(imply, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // Verify the root OR node
        assert!(
            matches!(entry.kind(), ExprKind::Or),
            "Root should be an OR node, found: {:?}",
            entry.kind()
        );
        assert_eq!(
            entry.children().len(),
            2,
            "The resulting OR node should have 2 children"
        );

        // Verify that the result is strictly (or (not A) B)
        let mut has_not_a = false;
        let mut has_b = false;

        for &child_id in entry.children() {
            let child = store.fetch(child_id)?;
            match child.kind() {
                ExprKind::Not => {
                    // The child of the NOT node should be the atom A
                    let grand_child_id = child
                        .children()
                        .get(0)
                        .copied()
                        .expect("NOT node should have a child");
                    let grand_child = store.fetch(grand_child_id)?;

                    if matches!(grand_child.kind(), ExprKind::AtomicFormula(_)) {
                        has_not_a = true;
                    }
                }
                ExprKind::AtomicFormula(_) => {
                    has_b = true;
                }
                _ => {}
            }
        }

        assert!(
            has_not_a,
            "Should have simplified the triple negation down to (not A)"
        );
        assert!(has_b, "Should have kept B as the second disjunct");

        Ok(())
    }

    /// Implication with a complex consequent:
    ///
    /// Input: (imply A (and B C))
    /// Expected: (or (not A) (and B C))
    #[test]
    fn test_normalize_with_and_or_nodes() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Create Atoms: A=1, B=2, C=3
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let c = builder.atomic_formula(PredicateSymbolId::from(3), &[], skel);

        // Construct the complex consequent: (and B C)
        let and_bc = builder.and(&[b, c]);
        // Construct the initial implication: (imply A (and B C))
        let imply = builder.imply(a, and_bc);

        // 3. Normalize (is_durative = false, is_effect = false)
        let root_after = normalize(imply, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // 1. Verify root is an OR node
        assert!(
            matches!(entry.kind(), ExprKind::Or),
            "Root should be an OR node, found: {:?}",
            entry.kind()
        );
        assert_eq!(entry.children().len(), 2);

        // 2. Check children: one should be (not A), the other should be (and B C)
        let mut has_negated_a = false;
        let mut has_and_bc = false;

        for &child_id in entry.children() {
            let child = store.fetch(child_id)?;
            match child.kind() {
                ExprKind::Not => {
                    let grandchild_id = child
                        .children()
                        .get(0)
                        .copied()
                        .expect("NOT node should have a child");
                    let grandchild = store.fetch(grandchild_id)?;
                    if matches!(grandchild.kind(), ExprKind::AtomicFormula(_)) {
                        has_negated_a = true;
                    }
                }
                ExprKind::And => {
                    if child.children().len() == 2 {
                        has_and_bc = true;
                    }
                }
                _ => {}
            }
        }

        assert!(has_negated_a, "Missing negated antecedent (not A)");
        assert!(
            has_and_bc,
            "Missing original consequent structure (and B C)"
        );

        Ok(())
    }

    /// Normalization with nested quantifiers using typed variables:
    ///
    /// Input: (imply (forall (?X - T1) (A)) (exists (?Y - T2) (B)))
    /// Expected Output (with quantifier elimination and NNF): (or B (not A))
    #[test]
    fn test_normalize_with_quantifiers() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Prepare variables and lists
        let var_x = builder.typed_variable(1, [101]); // ?X - T1
        let list_x = builder.typed_variable_list([var_x]);

        let var_y = builder.typed_variable(2, [102]); // ?Y - T2
        let list_y = builder.typed_variable_list([var_y]);

        // 2. Build atomic formulas
        let skel = AtomSkeletonId::from(0);
        let atomic_a = builder.atomic_formula(PredicateSymbolId::from(3), &[], skel);
        let atomic_b = builder.atomic_formula(PredicateSymbolId::from(4), &[], skel);

        // 3. Build quantifiers
        let forall_node = builder.forall(list_x, atomic_a)?;
        let exists_node = builder.exists(list_y, atomic_b)?;

        // 4. Build implication: (imply (forall...) (exists...))
        let imply = builder.imply(forall_node, exists_node);

        // 5. Normalization (is_durative = false, is_effect = false)
        let root_after = normalize(imply, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // Verify the root is an OR node
        assert!(
            matches!(entry.kind(), ExprKind::Or),
            "Root should be an OR node, found: {:?}",
            entry.kind()
        );
        assert_eq!(
            entry.children().len(),
            2,
            "Root OR should have exactly 2 children"
        );

        // Verify that the unused quantifiers were pruned, leaving B and (not A)
        let mut has_positive_b = false;
        let mut has_negated_a = false;

        for &child_id in entry.children() {
            let child = store.fetch(child_id)?;
            match child.kind() {
                ExprKind::AtomicFormula(_) => {
                    // This should be the simplified consequent B
                    has_positive_b = true;
                }
                ExprKind::Not => {
                    // This should be the negated antecedent (not A)
                    if let Some(&grandchild_id) = child.children().get(0) {
                        let grandchild = store.fetch(grandchild_id)?;
                        if matches!(grandchild.kind(), ExprKind::AtomicFormula(_)) {
                            has_negated_a = true;
                        }
                    }
                }
                _ => {}
            }
        }

        assert!(
            has_positive_b,
            "The unused 'exists' should be pruned, leaving the atomic consequent"
        );
        assert!(
            has_negated_a,
            "The unused 'forall' should be pruned, leaving the negated atomic antecedent"
        );

        Ok(())
    }

    /// Test: When with empty AND condition and non-trivial effect
    ///
    /// Input: (when (and) (and A B C))
    /// Process: (when True Effect) -> Effect
    /// Expected Output: (and A B C)
    #[test]
    fn test_when_empty_and_complex_effect() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Prepare atomic formulas: A=1, B=2, C=3
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let c = builder.atomic_formula(PredicateSymbolId::from(3), &[], skel);

        // 2. Build the effect: (and A B C)
        let effect = builder.and(&[a, b, c]);

        // 3. Build the condition: (and) -> True
        let condition = builder.and(&[]);

        // 4. Build the WHEN node
        let when_node = builder.when(condition, effect);

        // 5. Normalization (is_durative = false, is_effect = false)
        // The normalizer should detect that the condition is always true
        // and replace the WHEN node directly with its effect.
        let root_after = normalize(when_node, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root should now be the AND node of the effect
        assert!(
            matches!(entry.kind(), ExprKind::And),
            "The WHEN node should be simplified to its effect if the condition is True, found: {:?}",
            entry.kind()
        );

        // Verify that the 3 children (A, B, C) are present
        let children = entry.children();
        assert_eq!(
            children.len(),
            3,
            "The resulting AND node should have 3 children"
        );

        for &child_id in children {
            let child_node = store.fetch(child_id)?;
            assert!(
                matches!(child_node.kind(), ExprKind::AtomicFormula(_)),
                "Child node should be an AtomicFormula, found: {:?}",
                child_node.kind()
            );
        }

        Ok(())
    }

    /// Test: When with empty OR condition and multiple AND effect
    ///
    /// Input: (when (or) (and X Y Z))
    /// Process: (when False Effect) -> True (Empty And)
    /// Expected Output: (and)
    #[test]
    fn test_when_empty_or_complex_effect() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Prepare atomic formulas: X=1, Y=2, Z=3
        let skel = AtomSkeletonId::from(0);
        let x = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let y = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let z = builder.atomic_formula(PredicateSymbolId::from(3), &[], skel);

        // 2. Build the effect: (and X Y Z)
        let effect = builder.and(&[x, y, z]);

        // 3. Build the condition: (or) -> False
        let condition = builder.or(&[]);

        // 4. Build the WHEN node
        let when_node = builder.when(condition, effect);

        // 5. Normalization (is_durative = false, is_effect = false)
        // Since (or) is False, the conditional effect can never trigger.
        // It must be replaced by an empty effect (and).
        let root_after = normalize(when_node, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root must be an empty AND
        assert!(
            matches!(entry.kind(), ExprKind::And),
            "A conditional effect with a False condition must become an empty (and), found: {:?}",
            entry.kind()
        );

        assert_eq!(
            entry.children().len(),
            0,
            "The effect should have been totally removed, leaving 0 children"
        );

        Ok(())
    }

    /// Test: When with condition equal to complex effect
    ///
    /// Input: (when (and A B) (and A B))
    /// Process: If the condition implies the effect (and both are atoms/conjunctions),
    ///          the effect is redundant.
    /// Expected Output: (and)
    #[test]
    fn test_when_condition_equal_effect() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Prepare atomic formulas: A=1, B=2
        let skel = AtomSkeletonId::from(0);
        let a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);

        // 2. Build the condition: (and A B)
        let condition = builder.and(&[a, b]);

        // 3. Build the effect: (and A B)
        // Note: Recreating the structures ensures the normalizer
        // compares content rather than just pointer addresses.
        let a2 = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let b2 = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let effect = builder.and(&[a2, b2]);

        // 4. Build the WHEN node
        let when_node = builder.when(condition, effect);

        // 5. Normalization (is_durative = false, is_effect = false)
        // The effect (and A B) is already satisfied if the condition (and A B) is true.
        let root_after = normalize(when_node, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root must be an empty AND (null effect)
        assert!(
            matches!(entry.kind(), ExprKind::And),
            "An effect identical to its condition is redundant and should be simplified, found: {:?}",
            entry.kind()
        );

        assert_eq!(
            entry.children().len(),
            0,
            "The redundant effect should have been removed, leaving 0 children"
        );

        Ok(())
    }

    /// Test: When with non-trivial condition and empty AND effect
    ///
    /// Input: (when (and A B) (and))
    /// Process: A conditional effect that does nothing is itself a no-op.
    /// Expected Output: (and)
    #[test]
    fn test_when_nontrivial_condition_empty_effect() -> Result<(), NormalizationError> {
        let mut store = ExprStore::new();
        let mut scratch = Scratchpad::with_capacity(64);
        let mut builder = ExprBuilder::new(&mut store);

        // 1. Prepare the complex condition: (and A B)
        let skel = AtomSkeletonId::from(0);
        let cond_a = builder.atomic_formula(PredicateSymbolId::from(1), &[], skel);
        let cond_b = builder.atomic_formula(PredicateSymbolId::from(2), &[], skel);
        let condition = builder.and(&[cond_a, cond_b]);

        // 2. Prepare the empty effect: (and)
        let empty_and = builder.and(&[]);

        // 3. Build the WHEN node: (when (and A B) (and))
        let when_node = builder.when(condition, empty_and);

        // 4. Normalization (is_durative = false, is_effect = false)
        // Since the effect is empty, the condition no longer needs to be evaluated.
        // The WHEN node should be simplified into a simple empty (and).
        let root_after = normalize(when_node, &mut store, &mut scratch, false, false)?;

        // --- VALIDATION ---
        let entry = store.fetch(root_after)?;

        // The root must be an empty AND
        assert!(
            matches!(entry.kind(), ExprKind::And),
            "A conditional effect with no effect should be removed, found: {:?}",
            entry.kind()
        );

        assert_eq!(
            entry.children().len(),
            0,
            "The tree should contain no children (empty effect)"
        );

        Ok(())
    }
}
