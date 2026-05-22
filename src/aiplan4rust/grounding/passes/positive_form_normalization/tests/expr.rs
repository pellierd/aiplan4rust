use super::*;
use crate::aiplan4rust::grounding::passes::positive_form_normalization::expr::to_pnf;
use crate::aiplan4rust::lang::CompareOp;
use crate::aiplan4rust::lir::old::expr::builder::ExprBuilder;
use crate::aiplan4rust::lir::old::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::old::expr::{ExprContent, ExprKind};
use crate::aiplan4rust::tree::NodeId;

/// **Test Goal**: Verify the structural transformation of a negated atom into a single negated LIR node (Logical Mode).
///
/// **Input**:
/// - A `Not` node pointing to an `AtomicFormula` (Skeleton ID: 500).
/// - `is_effect = false` (Logical/Precondition mode).
///
/// **Expected Output**:
/// - The root node is transformed into an `AtomicFormula`.
/// - The internal `AtomSkeletonId` has its MSB (negation bit) set to `true`.
/// - The `negated_atoms` vector contains the negated Skeleton ID.
#[test]
fn test_encode_simple_atom_negation_logical() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();
    let mut dfs_stack = Vec::with_capacity(16);

    // 1. Setup: (not (at-robot r1))
    let atom = builder.atomic_formula_with_skeleton(1, vec![], 500);
    let not_node = builder.not(atom);
    builder.set_root(not_node)?;
    let mut expr = builder.finish();

    // 2. Transformation (is_effect = false)
    to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        false,
    )
    .expect("PNF encoding failed");

    // 3. Validation
    let root = expr.try_root_node()?;
    assert_eq!(
        root.kind(),
        ExprKind::AtomicFormula,
        "Logical 'Not' should be absorbed"
    );

    if let ExprContent::AtomSkeleton(id) = root.content() {
        assert!(id.is_negated(), "The MSB bit (negation) should be true");
        assert_eq!(negated_atoms.len(), 1);
        assert_eq!(negated_atoms[0], *id);
    } else {
        panic!("The node content should be an AtomSkeleton");
    }
    Ok(())
}

/// **Test Goal**: Verify that negation nodes are preserved in effect trees (Delete-Relaxation).
///
/// **Input**:
/// - A `Not` node pointing to an `AtomicFormula`.
/// - `is_effect = true` (Effect mode).
///
/// **Expected Output**:
/// - The root node remains a `Not` kind.
/// - No atoms are collected in `negated_atoms`.
#[test]
fn test_encode_effect_negation_preservation() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();
    let mut dfs_stack = Vec::with_capacity(16);

    let atom = builder.atomic_formula_with_skeleton(1, vec![], 500);
    let not_node = builder.not(atom);
    builder.set_root(not_node)?;
    let mut expr = builder.finish();

    // 2. Transformation (is_effect = true pour le mode effet)
    to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        true,
    )
    .expect("PNF encoding failed");

    // 3. Validation
    let root = expr.try_root_node()?;
    assert_eq!(
        root.kind(),
        ExprKind::Not,
        "Effect 'Not' (Delete) should be preserved"
    );
    assert!(
        negated_atoms.is_empty(),
        "Delete effects should not populate negated_atoms"
    );

    Ok(())
}

/// **Test Goal**: Ensure that negations of comparisons are NOT absorbed and remain structural.
///
/// **Input**:
/// - A `Not` node pointing to a `Comparison` node (`= ?x ?y`).
/// - `is_effect = false` (Logical mode).
///
/// **Expected Output**:
/// - The tree structure remains `Not -> Comparison` (no absorption).
/// - The `negated_atoms` vector remains **empty** (comparisons are not atoms).
/// - Operands of the comparison (?x, ?y) are preserved.
#[test]
fn test_encode_comparison_stays_unchanged() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();
    let mut dfs_stack = Vec::with_capacity(16);

    // 1. Creation of operands for the comparison
    let var_x = builder.variable(10);
    let var_y = builder.variable(11);

    // 2. Setup: (not (= ?x ?y))
    let comp = builder.comparison(CompareOp::Equal, var_x, var_y);
    let not_node = builder.not(comp);

    builder.set_root(not_node)?;
    let mut expr = builder.finish();

    let initial_root_id = expr.try_root_id()?;

    // 3. Transformation
    // On passe false car une comparaison est une condition logique (precond/goal/when-cond).
    to_pnf(
        initial_root_id,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        false,
    )?;

    // 4. Validation: Structure preservation
    let root = expr.try_root_node()?;
    assert_eq!(
        root.kind(),
        ExprKind::Not,
        "Root should still be a Not node for comparisons"
    );

    let child_id = root.children()[0];
    let child = expr.try_node(child_id)?;
    assert_eq!(
        child.kind(),
        ExprKind::Comparison,
        "Child should still be a Comparison node"
    );

    // Verify that operands (?x and ?y) are still attached
    assert_eq!(
        child.children().len(),
        2,
        "Comparison should still have 2 children"
    );

    // 5. Validation: Collection (Side-effect check)
    assert!(
        negated_atoms.is_empty(),
        "Negated comparisons must NOT be added to negated_atoms"
    );

    Ok(())
}

/// **Test Goal**: Ensure the encoder strictly enforces the transformation pipeline
/// by rejecting unsupported nodes (like `Imply`) under a `Not`.
///
/// **Input**:
/// - A `Not` node pointing directly to an `Imply` node.
/// - `is_effect = false` (Logical mode).
///
/// **Expected Output**:
/// - The function returns an `Err(ExprOpError::InvalidExprNode)`.
/// - The process stops immediately, maintaining pipeline integrity.
/// - The `negated_atoms` vector remains empty as the operation failed.
#[test]
fn test_detect_unsupported_node_under_not() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();

    // 1. Setup: (not (imply A B))
    // 'Imply' should have been removed by 'eliminate_imply' before this pass.
    let a = builder.atomic_formula_with_skeleton(1, vec![], 100);
    let b = builder.atomic_formula_with_skeleton(2, vec![], 101);
    let imply = builder.imply(a, b);
    let not_node = builder.not(imply);

    builder.set_root(not_node)?;
    let mut expr = builder.finish();

    // 2. Transformation: Attempting to encode an invalid PNF structure.
    // On utilise la pile de tuples (NodeId, bool) comme défini dans ta logique.
    let mut dfs_stack = Vec::with_capacity(16);
    let result = to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        false, // Mode logique
    );

    // 3. Validation: The function must return an Err instead of panicking or succeeding.
    assert!(
        result.is_err(),
        "Encoding should fail when an unsupported 'Imply' is found under a 'Not'"
    );

    // Safety check: no atoms should be partially collected on failure.
    assert!(negated_atoms.is_empty());

    Ok(())
}

/// **Test Goal**: Ensure the encoder rejects double negations (`NOT NOT`), enforcing
/// that the tree has been simplified prior to PNF encoding.
///
/// **Input**:
/// - A `Not` node pointing to another `Not` node: `(not (not A))`.
/// - `is_effect = false` (Logical mode).
///
/// **Expected Output**:
/// - The function returns an `Err(ExprOpError::InvalidExprNode)`.
/// - This confirms that the encoding pass relies on a prior simplification step.
#[test]
fn test_detect_double_negation_failure() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();

    // 1. Setup: (not (not A))
    // This structure should have been cleaned up by a simplification pass.
    let a = builder.atomic_formula_with_skeleton(1, vec![], 100);
    let not_inner = builder.not(a);
    let not_outer = builder.not(not_inner);

    builder.set_root(not_outer).unwrap();
    let mut expr = builder.finish();

    // 2. Transformation
    // On utilise la pile DFS de tuples (NodeId, bool)
    let mut dfs_stack: Vec<(NodeId, bool)> = Vec::with_capacity(16);
    let result = to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        false, // Mode logique (car on teste une structure de condition)
    );

    // 3. Validation: Ensure it returns an Error instead of trying to process it.
    assert!(
        result.is_err(),
        "Encoding should return an error for nested 'Not' nodes (double negation)"
    );

    // Ensure no partial data was collected.
    assert!(negated_atoms.is_empty());

    Ok(())
}

/// **Test Goal**: Verify PNF encoding on a mixed formula containing both an atom and a comparison.
///
/// **Input**:
/// - A conjunction: `(and (not (at-robot)) (not (= ?x ?y)))`.
/// - `is_effect = false` (Logical/Precondition mode).
///
/// **Expected Output**:
/// - The `(not (at-robot))` branch is absorbed: `Not` node disappears, `AtomicFormula` gets the MSB bit.
/// - The `(not (= ?x ?y))` branch remains structural: `Not` -> `Comparison`.
/// - The `negated_atoms` vector collects **only** the `at-robot` skeleton ID.
#[test]
fn test_mixed_complex_pnf() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();
    let mut dfs_stack = Vec::with_capacity(16);

    // 1. Setup Negated Atom: (not (at-robot))
    // Predicate ID: 1, Args: empty, Skeleton ID: 500
    let at = builder.atomic_formula_with_skeleton(1, vec![], 500);
    let not_at = builder.not(at);

    // 2. Setup Negated Comparison: (not (= ?x ?y))
    let var_x = builder.variable(10);
    let var_y = builder.variable(11);
    let comp = builder.comparison(CompareOp::Equal, var_x, var_y);
    let not_comp = builder.not(comp);

    // 3. Setup Root Conjunction: (and (not (at-robot)) (not (= ?x ?y)))
    let root_and = builder.and(vec![not_at, not_comp]);
    builder.set_root(root_and)?;
    let mut expr = builder.finish();

    // 4. Transformation: Lowering negations to PNF
    to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        false, // Mode logique
    )?;

    // 5. Validation: Tree Structure
    let root = expr.try_root_node()?;
    assert_eq!(root.kind(), ExprKind::And, "Root must remain an AND node");

    // First child: Not(At) must have been flattened into a negated AtomicFormula
    let first_child_id = root.children()[0];
    let first_child = expr.try_node(first_child_id)?;
    assert_eq!(
        first_child.kind(),
        ExprKind::AtomicFormula,
        "Atoms under Not should be absorbed"
    );

    if let ExprContent::AtomSkeleton(id) = first_child.content() {
        assert!(
            id.is_negated(),
            "The AtomicFormula skeleton should have its MSB bit set"
        );

        // Validation: Collection side-effect
        assert_eq!(negated_atoms.len(), 1, "Only the atom should be collected");
        assert_eq!(
            negated_atoms[0], *id,
            "The collected ID must match the negated atom's ID"
        );
    } else {
        panic!("First child content should be an AtomSkeleton");
    }

    // Second child: Not(Comparison) must remain unchanged (Not -> Comparison)
    let second_child_id = root.children()[1];
    let second_child = expr.try_node(second_child_id)?;
    assert_eq!(
        second_child.kind(),
        ExprKind::Not,
        "Negation above Comparison must be preserved"
    );

    let inner_comp_id = second_child.children()[0];
    let inner_comp = expr.try_node(inner_comp_id)?;
    assert_eq!(inner_comp.kind(), ExprKind::Comparison);
    assert_eq!(
        inner_comp.children().len(),
        2,
        "Comparison should still have its two variables"
    );

    Ok(())
}

/// **Test Goal**: Verify that the PNF encoder correctly traverses a quantifier
/// and transforms the inner negation, while preserving the atom's internal structure.
///
/// **Input**:
/// - A `Forall` node with `list_x` as metadata and `(not (at-robot ?x))` as child [0].
/// - `is_effect = false` (Logical mode).
///
/// **Expected Output**:
/// - Child [0] of the quantifier is transformed from `Not` to `AtomicFormula`.
/// - The negation bit (MSB) is set on the AtomicFormula's skeleton ID.
/// - The atom's arguments (variable ?x) are preserved at the correct index.
#[test]
fn test_pnf_traverses_quantifiers_with_correct_args() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();
    let mut dfs_stack = Vec::with_capacity(16);

    // 1. Setup Typed Variables (Internal Metadata / Signature)
    let var_x_sym = builder.typed_variable(2, &[102]);
    let list_x = builder.typed_variable_list(vec![var_x_sym]);

    // 2. Setup Arguments (Structural Links)
    let arg_x = builder.variable(2);

    // 3. Setup Negated Atom: (not (at-robot ?x))
    let at_x = builder.atomic_formula_with_skeleton(3, vec![arg_x], 500);
    let not_at = builder.not(at_x);

    // 4. Setup Forall: The formula body is the first child [0]
    let forall = builder.forall(list_x, not_at);
    builder.set_root(forall)?;
    let mut expr = builder.finish();

    // --- Transformation ---
    // On passe 'false' car un Forall est une condition logique.
    to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        false,
    )?;

    // --- Validation: Quantifier Level ---
    let root = expr.try_root_node()?;
    assert_eq!(
        root.kind(),
        ExprKind::Forall,
        "Root must remain a Forall node"
    );

    // --- Validation: Atom Level ---
    let body_id = root.children()[0];
    let body_node = expr.try_node(body_id)?;
    assert_eq!(
        body_node.kind(),
        ExprKind::AtomicFormula,
        "The inner Not should be absorbed"
    );

    if let ExprContent::AtomSkeleton(id) = body_node.content() {
        assert!(id.is_negated(), "The atom's MSB bit must be set to true");
        assert_eq!(
            negated_atoms.len(),
            1,
            "The nested negated atom should be collected"
        );
        assert_eq!(negated_atoms[0], *id);
    } else {
        panic!("Expected AtomSkeleton content in the body node");
    }

    // --- Validation: Arguments Integrity ---
    // The argument ?x should be preserved at the second index (index 1).
    assert_eq!(
        body_node.children()[1],
        arg_x,
        "The variable argument ?x must be preserved"
    );

    Ok(())
}

/// **Test Goal**: Verify that the encoder detects and rejects an `AtomicFormula`
/// that already has its negation bit (MSB) set when encountered under a `Not` node.
///
/// **Input**:
/// - A `Not` node pointing to an `AtomicFormula` whose `AtomSkeletonId` already has `is_negated() == true`.
/// - `is_effect = false` (Logical mode).
///
/// **Expected Output**:
/// - The function returns `Err(ExprOpError::InvalidExprNode)`.
/// - This prevents accidental "double-bit-flipping" which would lead to inconsistent LIR states.
#[test]
fn test_detect_forbidden_double_negation_in_bit() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();
    let mut dfs_stack = Vec::with_capacity(16);

    // 1. Create an atom and setup (not (already_negated_atom))
    let atom_id = builder.atomic_formula_with_skeleton(1, vec![], 500);
    let not_node = builder.not(atom_id);
    builder.set_root(not_node).unwrap();
    let mut expr = builder.finish();

    // Force the bit to true manually to simulate an inconsistent/corrupted state
    if let ExprContent::AtomSkeleton(ref mut id) = expr.try_node_mut(atom_id)?.content_mut() {
        id.set_negated(true);
    }

    // 2. Transformation: The encoder finds a 'Not' above a node that is already bit-negated.
    let result = to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        false, // Mode logique
    );

    // 3. Validation: Ensure it returns an Error instead of flipping the bit back to positive.
    assert!(
        result.is_err(),
        "Should return Err when a 'Not' is found above an already bit-negated atom"
    );

    // The collection should be empty or reflect the failure state
    assert!(
        negated_atoms.is_empty(),
        "No atoms should be indexed upon failure"
    );

    Ok(())
}

/// **Test Goal**: Verify that the PNF encoder successfully descends into quantifier nodes
/// (`Forall`/`Exists`) to reach and transform nested negations.
///
/// **Input**:
/// - A `Forall` node: `(forall (?x) (not (at ?x)))`.
/// - `is_effect = false` (Logical/Condition mode).
///
/// **Expected Output**:
/// - The `Forall` node remains as the root.
/// - The child `Not` node is absorbed into the `AtomicFormula`.
/// - The `negated_atoms` vector contains the `AtomSkeletonId` (500) with its negation bit set.
#[test]
fn test_pnf_descends_into_quantifiers() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();
    let mut dfs_stack = Vec::with_capacity(16);

    // 1. Declare the typed variable for the quantifier scope
    // ?x - Type ID 102 (example)
    let var_x_sym = builder.typed_variable(10, &[102]);
    let var_list = builder.typed_variable_list(vec![var_x_sym]);

    // 2. Create the variable node for the atom arguments
    let arg_x = builder.variable(10);

    // 3. Setup: (forall (?x) (not (at ?x)))
    let at_x = builder.atomic_formula_with_skeleton(1, vec![arg_x], 500);
    let not_at = builder.not(at_x);
    let forall = builder.forall(var_list, not_at);

    builder.set_root(forall)?;
    let mut expr = builder.finish();

    // 4. Transformation
    // On passe 'false' car un Forall est une condition (precond/goal).
    to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        false,
    )?;

    // 5. Validation
    let root = expr.try_root_node()?;
    assert_eq!(root.kind(), ExprKind::Forall);

    // Get the quantifier's body (the first child in this LIR)
    let child_id = root.children()[0];
    let child = expr.try_node(child_id)?;

    // The 'Not' node must be gone, replaced by the negated AtomicFormula
    assert_eq!(
        child.kind(),
        ExprKind::AtomicFormula,
        "The Not node should have been absorbed into the AtomicFormula"
    );

    if let ExprContent::AtomSkeleton(id) = child.content() {
        assert!(
            id.is_negated(),
            "The MSB bit of the AtomSkeletonId should be true"
        );

        // Verification of side-effect collection
        assert_eq!(
            negated_atoms.len(),
            1,
            "The nested negated atom should be indexed"
        );
        assert_eq!(negated_atoms[0], *id);
    } else {
        panic!("Child content should be an AtomSkeleton");
    }

    Ok(())
}

/// **Test Goal**: Verify that the PNF encoder can handle very deep expression trees
/// without causing a stack overflow, ensuring the iterative DFS approach is robust.
///
/// **Input**:
/// - A leaf `AtomicFormula` (ID 500) wrapped in a `Not`, then nested under 1,000 `And` nodes.
/// - `is_effect = false` (Logical mode).
///
/// **Expected Output**:
/// - The function completes successfully (iterative stack vs recursion).
/// - The leaf node is correctly transformed into a negated `AtomicFormula`.
/// - The `negated_atoms` vector collects the ID 500 despite the depth.
#[test]
fn test_pnf_deep_nesting() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();
    let mut dfs_stack = Vec::with_capacity(16);

    // 1. Create a leaf atom
    let atom = builder.atomic_formula_with_skeleton(1, vec![], 500);

    // 2. Wrap it in a 'Not' node
    let mut current = builder.not(atom);

    // 3. Create a deep chain of 1,000 'And' nodes to test iterative traversal
    // This is a stress test for stack safety.
    for _ in 0..1000 {
        current = builder.and(vec![current]);
    }

    builder.set_root(current)?;
    let mut expr = builder.finish();

    // 4. Transformation
    // Iterative traversal should handle this easily where recursion would fail.
    to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        false, // Mode logique
    )?;

    // 5. Validation: Trace down to the atom to ensure it was negated
    let mut curr_id = expr.try_root_id()?;
    for _ in 0..1000 {
        let node = expr.try_node(curr_id)?;
        assert_eq!(node.kind(), ExprKind::And);
        curr_id = node.children()[0];
    }

    let final_node = expr.try_node(curr_id)?;
    assert_eq!(
        final_node.kind(),
        ExprKind::AtomicFormula,
        "Leaf should be an atom now"
    );

    if let ExprContent::AtomSkeleton(id) = final_node.content() {
        assert!(
            id.is_negated(),
            "The deep atom should be successfully negated"
        );

        // 6. Validation: Collection across depth
        assert_eq!(
            negated_atoms.len(),
            1,
            "The atom should be collected regardless of depth"
        );
        assert_eq!(negated_atoms[0], *id);
    } else {
        panic!("Leaf content should be an AtomSkeleton");
    }

    Ok(())
}

/// **Test Goal**: Verify that the PNF encoder correctly traverses multiple nested
/// layers of different quantifiers (`Forall` and `Exists`) to reach the leaf nodes.
///
/// **Input**:
/// - A nested structure: `(forall (?x) (exists (?y) (not (at ?x ?y))))`.
/// - `is_effect = false` (Logical mode).
///
/// **Expected Output**:
/// - Both quantifier nodes (`Forall`, `Exists`) are preserved in the tree.
/// - The `Not` node at the bottom is absorbed into the `AtomicFormula`.
/// - The `negated_atoms` vector contains the ID 500.
#[test]
fn test_pnf_traverses_multiple_quantifier_layers() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();
    let mut dfs_stack = Vec::with_capacity(16);

    // 1. Declare typed variables for different scopes
    let var_x = builder.typed_variable(1, &[100]);
    let var_y = builder.typed_variable(2, &[100]);
    let list_x = builder.typed_variable_list(vec![var_x]);
    let list_y = builder.typed_variable_list(vec![var_y]);

    // 2. Build the leaf: (not (at ?x ?y))
    let arg_x = builder.variable(1);
    let arg_y = builder.variable(2);
    let atom = builder.atomic_formula_with_skeleton(3, vec![arg_x, arg_y], 500);
    let not_atom = builder.not(atom);

    // 3. Nest quantifiers: (forall (?x) (exists (?y) (not (at ?x ?y))))
    let exists = builder.exists(list_y, not_atom);
    let forall = builder.forall(list_x, exists);

    builder.set_root(forall)?;
    let mut expr = builder.finish();

    // 4. Transformation
    // L'itératif DFS descend : Forall -> Exists -> Not (puis absorbe le Not)
    to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        false, // Mode logique
    )?;

    // 5. Validation : On vérifie que la feuille est maintenant une AtomicFormula négative
    let root = expr.try_root_node()?;
    assert_eq!(root.kind(), ExprKind::Forall);

    // Descend vers Exists
    let exists_id = root.children()[0];
    let exists_node = expr.try_node(exists_id)?;
    assert_eq!(exists_node.kind(), ExprKind::Exists);

    // Descend vers l'AtomicFormula (le nœud 'Not' doit avoir disparu)
    let final_atom_id = exists_node.children()[0];
    let final_atom = expr.try_node(final_atom_id)?;

    assert_eq!(
        final_atom.kind(),
        ExprKind::AtomicFormula,
        "Le nœud 'Not' devrait être absorbé"
    );

    if let ExprContent::AtomSkeleton(id) = final_atom.content() {
        assert!(
            id.is_negated(),
            "L'atome imbriqué doit avoir son bit MSB activé"
        );

        // 6. Validation : Collecte de l'effet de bord
        assert_eq!(
            negated_atoms.len(),
            1,
            "L'atome profond doit être collecté dans negated_atoms"
        );
        assert_eq!(negated_atoms[0], *id);
    } else {
        panic!("Contenu AtomSkeleton attendu");
    }

    Ok(())
}

/// **Test Goal**: Ensure the encoder enforces the PNF pipeline by rejecting complex
/// logical connectors (like `And`) found directly under a `Not` node.
///
/// **Input**:
/// - A `Not` node pointing to an `And` node: `(not (and A B))`.
/// - `is_effect = false` (Logical mode).
///
/// **Expected Output**:
/// - The function returns `Err(ExprOpError::InvalidExprNode)`.
/// - This confirms that De Morgan's laws must have been applied by `push_negation`
///   before calling the PNF encoder.
#[test]
fn test_detect_forbidden_complex_node_under_not() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();
    let mut dfs_stack: Vec<(NodeId, bool)> = Vec::with_capacity(16);

    // 1. Setup: (not (and A B))
    // This is illegal at this stage; it should have been transformed into (or (not A) (not B))
    let a = builder.atomic_formula_with_skeleton(1, vec![], 100);
    let b = builder.atomic_formula_with_skeleton(2, vec![], 101);
    let and_node = builder.and(vec![a, b]);
    let not_node = builder.not(and_node);

    builder.set_root(not_node)?;
    let mut expr = builder.finish();

    // 2. Transformation
    // The encoder must detect that 'And' is not a valid child for 'Not' in this pass.
    let result = to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
        false, // Mode logique
    );

    // 3. Validation
    assert!(
        result.is_err(),
        "Encoding should fail when a complex 'And' node is found under a 'Not'"
    );

    // Ensure no atoms were collected from the failing branch
    assert!(negated_atoms.is_empty());

    Ok(())
}
