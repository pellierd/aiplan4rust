use super::*;
use crate::aiplan4rust::lang::CompareOp;
use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
use crate::aiplan4rust::lir::expr::{ExprKind, ExprContent};
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::grounding::passes::positive_normal_form::expr::encode_to_pnf;

/// **Test Goal**: Verify the structural transformation of a negated atom into a single negated LIR node.
///
/// **Input**:
/// - A `Not` node pointing to an `AtomicFormula` (Skeleton ID: 500).
/// - An empty `Vec<AtomSkeletonId>` for collection.
///
/// **Expected Output**:
/// - The root node is transformed into an `AtomicFormula`.
/// - The internal `AtomSkeletonId` has its MSB (negation bit) set to `true`.
/// - The `negated_atoms` vector contains exactly one ID (the negated Skeleton ID 500).
#[test]
fn test_encode_simple_atom_negation() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();

    // 1. Setup: (not (at-robot r1))
    // We use atomic_formula_with_skeleton because the LIR requires an AtomSkeletonId.
    // Predicate ID: 1, Args: empty, Skeleton ID: 500.
    let atom = builder.atomic_formula_with_skeleton(1, vec![], 500);
    let not_node = builder.not(atom);

    builder.set_root(not_node)?;
    let mut expr = builder.finish();

    // 2. Transformation
    // We pass the reference to our collection vector to track negated literals.
    let mut dfs_stack = Vec::with_capacity(16);
    encode_to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack
    ).expect("PNF encoding failed");

    // 3. Validation: The 'Not' node should be replaced by the 'AtomicFormula' with MSB set.
    let root = expr.try_root_node()?;
    assert_eq!(root.kind(), ExprKind::AtomicFormula, "Root should have absorbed the negation");

    if let ExprContent::AtomSkeleton(id) = root.content() {
        // Check MSB bit
        assert!(id.is_negated(), "The MSB bit (negation) should be true");

        // Verify collection side-effect
        assert_eq!(negated_atoms.len(), 1, "One negated atom should have been collected");
        assert_eq!(negated_atoms[0], *id, "Collected ID must match the modified root ID");
    } else {
        panic!("The node content should be an AtomSkeleton");
    }

    Ok(())
}

/// **Test Goal**: Ensure that negations of comparisons are NOT absorbed and remain structural.
///
/// **Input**:
/// - A `Not` node pointing to a `Comparison` node (`= ?x ?y`).
/// - An empty `Vec<AtomSkeletonId>` for collection.
///
/// **Expected Output**:
/// - The tree structure remains `Not -> Comparison` (no absorption).
/// - The `negated_atoms` vector remains **empty** (comparisons are not atoms).
/// - Operands of the comparison (?x, ?y) are preserved.
#[test]
fn test_encode_comparison_stays_unchanged() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();

    // 1. Creation of operands for the comparison
    // We create two variables ?x (ID 10) and ?y (ID 11)
    let var_x = builder.variable(10);
    let var_y = builder.variable(11);

    // 2. Setup: (not (= ?x ?y))
    let comp = builder.comparison(CompareOp::Equal, var_x, var_y);
    let not_node = builder.not(comp);

    builder.set_root(not_node)?;
    let mut expr = builder.finish();

    let initial_root_id = expr.try_root_id()?;

    // 3. Transformation
    // Even with the vector provided, nothing should be collected here.
    let mut dfs_stack = Vec::with_capacity(16);
    encode_to_pnf(initial_root_id, &mut expr, &mut negated_atoms, &mut dfs_stack)?;

    // 4. Validation: Structure preservation
    let root = expr.try_root_node()?;
    assert_eq!(root.kind(), ExprKind::Not, "Root should still be a Not node");

    let child_id = root.children()[0];
    let child = expr.try_node(child_id)?;
    assert_eq!(child.kind(), ExprKind::Comparison, "Child should be a Comparison node");

    // Verify that operands (?x and ?y) are still attached
    assert_eq!(child.children().len(), 2, "Comparison should still have 2 children");
    assert_eq!(child.children()[0], var_x);
    assert_eq!(child.children()[1], var_y);

    // 5. Validation: Collection (Side-effect check)
    assert!(negated_atoms.is_empty(), "Negated comparisons should not be added to negated_atoms");

    Ok(())
}

/// **Test Goal**: Ensure the encoder strictly enforces the transformation pipeline
/// by rejecting unsupported nodes (like `Imply`) under a `Not`.
///
/// **Input**:
/// - A `Not` node pointing directly to an `Imply` node.
/// - An empty `Vec<AtomSkeletonId>` for collection.
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

    builder.set_root(not_node).unwrap();
    let mut expr = builder.finish();

    // 2. Transformation: Attempting to encode an invalid PNF structure.
    let mut dfs_stack = Vec::with_capacity(16);
    let result = encode_to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack,
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
/// - An empty `Vec<AtomSkeletonId>` for collection.
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
    // In PNF encoding, we never expect to see a 'Not' node under another 'Not' node.
    let mut dfs_stack = Vec::with_capacity(16);
    let result = encode_to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack
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
/// - An empty `Vec<AtomSkeletonId>` for collection.
///
/// **Expected Output**:
/// - The `(not (at-robot))` branch is absorbed: `Not` node disappears, `AtomicFormula` gets the MSB bit.
/// - The `(not (= ?x ?y))` branch remains structural: `Not` -> `Comparison`.
/// - The `negated_atoms` vector collects **only** the `at-robot` skeleton ID.
/// - The comparison operands (`?x`, `?y`) remain intact.
#[test]
fn test_mixed_complex_pnf() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();

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
    let mut dfs_stack = Vec::with_capacity(16);
    encode_to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack
    )?;

    // 5. Validation: Tree Structure
    let root = expr.try_root_node()?;
    assert_eq!(root.kind(), ExprKind::And, "Root must remain an AND node");

    // First child: Not(At) must have been flattened into a negated AtomicFormula
    let first_child_id = root.children()[0];
    let first_child = expr.try_node(first_child_id)?;
    assert_eq!(first_child.kind(), ExprKind::AtomicFormula, "Atoms under Not should be absorbed");

    if let ExprContent::AtomSkeleton(id) = first_child.content() {
        assert!(id.is_negated(), "The AtomicFormula skeleton should have its MSB bit set");

        // Validation: Collection side-effect
        assert_eq!(negated_atoms.len(), 1, "Only the atom should be collected");
        assert_eq!(negated_atoms[0], *id, "The collected ID must match the negated atom's ID");
    } else {
        panic!("First child content should be an AtomSkeleton");
    }

    // Second child: Not(Comparison) must remain unchanged (Not -> Comparison)
    let second_child_id = root.children()[1];
    let second_child = expr.try_node(second_child_id)?;
    assert_eq!(second_child.kind(), ExprKind::Not, "Negation above Comparison must be preserved");

    let inner_comp_id = second_child.children()[0];
    let inner_comp = expr.try_node(inner_comp_id)?;
    assert_eq!(inner_comp.kind(), ExprKind::Comparison);
    assert_eq!(inner_comp.children().len(), 2, "Comparison should still have its two variables");

    Ok(())
}

/// **Test Goal**: Verify that the PNF encoder correctly traverses a quantifier
/// and transforms the inner negation, while preserving the atom's internal structure.
///
/// **Input**:
/// - A `Forall` node with `list_x` as metadata and `(not (at-robot ?x))` as child [0].
/// - An empty `Vec<AtomSkeletonId>` for collection.
///
/// **Expected Output**:
/// - Child [0] of the quantifier is transformed from `Not` to `AtomicFormula`.
/// - The negation bit (MSB) is set on the AtomicFormula's skeleton ID.
/// - The atom's arguments (variable ?x) are preserved at the correct index.
#[test]
fn test_pnf_traverses_quantifiers_with_correct_args() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();

    // 1. Setup Typed Variables (Internal Metadata / Signature)
    // Variable ID: 2, Type ID: 102
    let var_x_sym = builder.typed_variable(2, &[102]);
    let list_x = builder.typed_variable_list(vec![var_x_sym]);

    // 2. Setup Arguments (Structural Links)
    // Variable reference ID: 2
    let arg_x = builder.variable(2);

    // 3. Setup Negated Atom: (not (at-robot ?x))
    // Predicate ID: 3, Args: [?x], Skeleton ID: 500
    let at_x = builder.atomic_formula_with_skeleton(3, vec![arg_x], 500);
    let not_at = builder.not(at_x);

    // 4. Setup Forall: The formula body is the first child [0]
    let forall = builder.forall(list_x, not_at);

    builder.set_root(forall)?;
    let mut expr = builder.finish();

    // --- Transformation ---
    // The DFS must descend through the Forall node to find and absorb the Not node.
    let mut dfs_stack = Vec::with_capacity(16);
    encode_to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack
    )?;

    // --- Validation: Quantifier Level ---
    let root = expr.try_root_node()?;
    assert_eq!(root.kind(), ExprKind::Forall, "Root must remain a Forall node");

    // --- Validation: Atom Level ---
    // In this LIR, the body of the quantifier is the first child (index 0).
    let body_id = root.children()[0];
    let body_node = expr.try_node(body_id)?;

    // The 'Not' node should have been absorbed into the AtomicFormula
    assert_eq!(body_node.kind(), ExprKind::AtomicFormula, "The inner Not should be absorbed into the atom");

    // Check if the negation bit is correctly set in the skeleton ID
    if let ExprContent::AtomSkeleton(id) = body_node.content() {
        assert!(id.is_negated(), "The atom's MSB bit must be set to true");

        // Verify that the negated atom was correctly indexed for Closed World Assumption
        assert_eq!(negated_atoms.len(), 1, "The nested negated atom should be collected");
        assert_eq!(negated_atoms[0], *id);
    } else {
        panic!("Expected AtomSkeleton content in the body node");
    }

    // --- Validation: Arguments Integrity ---
    // The AtomicFormula node has 2 children: [PredicateID, ArgumentID].
    // This explains why len is 2 instead of 1.
    assert_eq!(body_node.children().len(), 2, "Atom should have 2 structural children (Predicate + Arg)");

    // The argument ?x should be preserved at the second index (index 1).
    assert_eq!(body_node.children()[1], arg_x, "The variable argument ?x must be preserved at index 1");

    Ok(())
}

/// **Test Goal**: Verify that the encoder detects and rejects an `AtomicFormula`
/// that already has its negation bit (MSB) set when encountered under a `Not` node.
///
/// **Input**:
/// - A `Not` node pointing to an `AtomicFormula` whose `AtomSkeletonId` already has `is_negated() == true`.
/// - An empty `Vec<AtomSkeletonId>` for collection.
///
/// **Expected Output**:
/// - The function returns `Err(ExprOpError::InvalidExprNode)`.
/// - This prevents accidental "double-bit-flipping" which would lead to inconsistent LIR states.
#[test]
fn test_detect_forbidden_double_negation_in_bit() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();

    // 1. Create an atom and manually set its negation bit
    let atom_id = builder.atomic_formula_with_skeleton(1, vec![], 500);

    // We simulate a state where the atom is already negated at the bit level
    // before the encoder even sees it under a 'Not' node.
    {
        // Internal trick for the test: reach into the builder or a temp expr
        // to flip the bit if your API allows, or simulate a tree where
        // a previous partial pass already touched the node.
    }

    // 2. Setup: (not (negated_atom))
    let not_node = builder.not(atom_id);
    builder.set_root(not_node).unwrap();
    let mut expr = builder.finish();

    // Force the bit to true for the sake of the test if not already done
    if let ExprContent::AtomSkeleton(ref mut id) = expr.try_node_mut(atom_id)?.content_mut() {
        id.set_negated(true);
    }

    // 3. Transformation: The encoder finds a 'Not' above a node that is already bit-negated.
    let mut dfs_stack = Vec::with_capacity(16);
    let result = encode_to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack
    );

    // 4. Validation: Ensure it returns an Error instead of flipping the bit back to positive.
    assert!(
        result.is_err(),
        "Should return Err when a 'Not' is found above an already bit-negated atom"
    );

    // The collection should be empty or reflect the failure state
    assert!(negated_atoms.is_empty(), "No atoms should be indexed upon failure");

    Ok(())
}

/// **Test Goal**: Verify that the PNF encoder successfully descends into quantifier nodes
/// (`Forall`/`Exists`) to reach and transform nested negations.
///
/// **Input**:
/// - A `Forall` node: `(forall (?x) (not (at ?x)))`.
/// - An empty `Vec<AtomSkeletonId>` for collection.
///
/// **Expected Output**:
/// - The `Forall` node remains as the root.
/// - The child `Not` node is absorbed into the `AtomicFormula`.
/// - The `negated_atoms` vector contains the `AtomSkeletonId` (500) with its negation bit set.
#[test]
fn test_pnf_descends_into_quantifiers() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();

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
    // We expect this to succeed as it traverses the Forall to reach the Not.
    let mut dfs_stack = Vec::with_capacity(16);
    encode_to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack
    )?;

    // 5. Validation
    let root = expr.try_root_node()?;
    assert_eq!(root.kind(), ExprKind::Forall);

    // Get the quantifier's body (the first child)
    let child_id = root.children()[0];
    let child = expr.try_node(child_id)?;

    // The 'Not' node must be gone, replaced by the negated AtomicFormula
    assert_eq!(
        child.kind(),
        ExprKind::AtomicFormula,
        "The Not node should have been absorbed into the AtomicFormula"
    );

    if let ExprContent::AtomSkeleton(id) = child.content() {
        assert!(id.is_negated(), "The MSB bit of the AtomSkeletonId should be true");

        // Verification of side-effect collection
        assert_eq!(negated_atoms.len(), 1, "The nested negated atom should be indexed");
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
/// - An empty `Vec<AtomSkeletonId>` for collection.
///
/// **Expected Output**:
/// - The function completes successfully (iterative stack vs recursion).
/// - The leaf node is correctly transformed into a negated `AtomicFormula`.
/// - The `negated_atoms` vector collects the ID 500 despite the depth.
#[test]
fn test_pnf_deep_nesting() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();

    // 1. Create a leaf atom
    let atom = builder.atomic_formula_with_skeleton(1, vec![], 500);

    // 2. Wrap it in a 'Not' node
    let mut current = builder.not(atom);

    // 3. Create a deep chain of 1000 'And' nodes to test iterative traversal
    // This is a stress test for stack safety.
    for _ in 0..1000 {
        current = builder.and(vec![current]);
    }

    builder.set_root(current)?;
    let mut expr = builder.finish();

    // 4. Transformation
    // Iterative traversal should handle this easily where recursion would fail.
    let mut dfs_stack = Vec::with_capacity(16);
    encode_to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack
    )?;

    // 5. Validation: Trace down to the atom to ensure it was negated
    let mut curr_id = expr.try_root_id()?;
    for _ in 0..1000 {
        let node = expr.try_node(curr_id)?;
        assert_eq!(node.kind(), ExprKind::And);
        curr_id = node.children()[0];
    }

    let final_node = expr.try_node(curr_id)?;
    assert_eq!(final_node.kind(), ExprKind::AtomicFormula);

    if let ExprContent::AtomSkeleton(id) = final_node.content() {
        assert!(id.is_negated(), "The deep atom should be successfully negated");

        // 6. Validation: Collection across depth
        assert_eq!(negated_atoms.len(), 1, "The atom should be collected regardless of depth");
        assert_eq!(negated_atoms[0], *id);
    }

    Ok(())
}

/// **Test Goal**: Verify that the PNF encoder correctly traverses multiple nested
/// layers of different quantifiers (`Forall` and `Exists`) to reach the leaf nodes.
///
/// **Input**:
/// - A nested structure: `(forall (?x) (exists (?y) (not (at ?x ?y))))`.
/// - An empty `Vec<AtomSkeletonId>` for collection.
///
/// **Expected Output**:
/// - Both quantifier nodes (`Forall`, `Exists`) are preserved in the tree.
/// - The `Not` node at the bottom is absorbed into the `AtomicFormula`.
/// - The `negated_atoms` vector contains the ID 500.
#[test]
fn test_pnf_traverses_multiple_quantifier_layers() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();

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
    // The iterative DFS should descend: Forall -> Exists -> Not
    let mut dfs_stack = Vec::with_capacity(16);
    encode_to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack
    )?;

    // 5. Validation: check if the leaf is now a negated AtomicFormula
    let root = expr.try_root_node()?;
    assert_eq!(root.kind(), ExprKind::Forall);

    // Descend to Exists
    let exists_id = root.children()[0];
    let exists_node = expr.try_node(exists_id)?;
    assert_eq!(exists_node.kind(), ExprKind::Exists);

    // Descend to the AtomicFormula (the Not node should have been absorbed)
    let final_atom_id = exists_node.children()[0];
    let final_atom = expr.try_node(final_atom_id)?;

    assert_eq!(final_atom.kind(), ExprKind::AtomicFormula, "The Not node should be gone");

    if let ExprContent::AtomSkeleton(id) = final_atom.content() {
        assert!(id.is_negated(), "The atom nested deep within quantifiers should be bit-negated");

        // 6. Validation: Side-effect collection
        assert_eq!(negated_atoms.len(), 1, "The deep atom must be collected in negated_atoms");
        assert_eq!(negated_atoms[0], *id);
    } else {
        panic!("Expected AtomSkeleton content");
    }

    Ok(())
}

/// **Test Goal**: Ensure the encoder enforces the PNF pipeline by rejecting complex
/// logical connectors (like `And`) found directly under a `Not` node.
///
/// **Input**:
/// - A `Not` node pointing to an `And` node: `(not (and A B))`.
/// - An empty `Vec<AtomSkeletonId>` for collection.
///
/// **Expected Output**:
/// - The function returns `Err(ExprOpError::InvalidExprNode)`.
/// - This confirms that De Morgan's laws must have been applied by `push_negation`
///   before calling the PNF encoder.
#[test]
fn test_detect_forbidden_complex_node_under_not() -> Result<(), ExprOpError> {
    let mut builder = ExprBuilder::new();
    let mut negated_atoms = Vec::new();

    // 1. Setup: (not (and A B))
    // This is illegal at this stage; it should have been (or (not A) (not B))
    let a = builder.atomic_formula_with_skeleton(1, vec![], 100);
    let b = builder.atomic_formula_with_skeleton(2, vec![], 101);
    let and_node = builder.and(vec![a, b]);
    let not_node = builder.not(and_node);

    builder.set_root(not_node).unwrap();
    let mut expr = builder.finish();

    // 2. Transformation
    // The encoder must detect that 'And' is not a valid child for 'Not' in PNF.
    let mut dfs_stack = Vec::with_capacity(16);
    let result = encode_to_pnf(
        expr.try_root_id()?,
        &mut expr,
        &mut negated_atoms,
        &mut dfs_stack
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
