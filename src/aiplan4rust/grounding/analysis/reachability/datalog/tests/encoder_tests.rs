use std::error::Error;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::encoder::DatalogEncoder;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::lang::{AtomSkeletonId, Type, TypeId, TypedList, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::expr::ExprBuilder;

/// Type alias for cleaner test signatures using the standard Error trait.
type TestResult = Result<(), Box<dyn Error>>;

/// Initialize a standardized execution environment for Datalog encoding tests.
///
/// # Setup Details
/// - **Encoder**: A `DatalogEncoder` initialized with an auxiliary predicate offset of 100
///   to avoid ID collisions with domain predicates.
/// - **Parameters**: A `TypedList` pre-populated with three default variables
///   (`?v0`, `?v1`, `?v2`) mapped to the root type.
///
/// # Output
/// - A fresh `DatalogEncoder` instance.
/// - A `TypedList` representing the available action parameters or variables.
fn setup_env() -> (DatalogEncoder, TypedList<VariableId, TypeId>) {
    // Starting auxiliary predicates at 100 makes debugging easier by
    // visually separating them from domain-defined predicates.
    let encoder = DatalogEncoder::new(100);
    let mut params = TypedList::new();

    // Create ?v0, ?v1, ?v2 as standard variables for logical expressions.
    for i in 0..3 {
        params.push(TypedSymbol::new(VariableId::from(i), Type::root()));
    }

    (encoder, params)
}

/// # Objective
/// Verify that complex, nested logical expressions are correctly flattened into
/// a chain of Datalog rules. This ensures that mixed operators (AND/OR)
/// result in correctly linked auxiliary predicates.
///
/// # Input
/// - Logic: `(AND (P10 ?v0) (OR (P11 ?v1) (P12 ?v2)))`
/// - Structure: A conjunction where one operand is a disjunction.
///
/// # Expected Output
/// - At least 3 Datalog rules:
///     - 2 rules for the `OR` branch (e.g., `P11 -> Aux_OR`, `P12 -> Aux_OR`).
///     - 1 rule for the root `AND` (e.g., `P10, Aux_OR -> Aux_ROOT`).
/// - A final auxiliary head that correctly represents the combined logic.
#[test]
fn test_complex_logical_flattening() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Define variables and atoms for: (AND (P10 ?v0) (OR (P11 ?v1) (P12 ?v2)))
    let v0 = builder.variable(0);
    let v1 = builder.variable(1);
    let v2 = builder.variable(2);

    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let p11 = builder.atomic_formula_with_skeleton(11, vec![v1], 11);
    let p12 = builder.atomic_formula_with_skeleton(12, vec![v2], 12);

    // Construct the nested tree
    let or_branch = builder.or(vec![p11, p12]);
    let root = builder.and(vec![p10, or_branch]);

    builder.set_root(root)?;
    let expr = builder.finish();

    let result = encoder.encode_expr(&expr, &mut rules, &params)?;

    // --- Industrial Requirement: Correct Rule Chaining ---
    // The OR branch must be decoupled into its own predicate to maintain
    // Datalog's Horn clause structure.
    assert!(
        rules.len() >= 3,
        "Expected at least 2 rules for the OR branch and 1 rule for the root AND"
    );

    // The final head must be an auxiliary predicate (typically with an ID >= 100
    // to distinguish from original domain predicates).
    let head = result.expect("Should return a final auxiliary atom");
    assert!(
        head.skeleton_id().as_usize() >= 100,
        "The root of a complex expression should be an auxiliary predicate"
    );

    Ok(())
}

/// # Objective
/// Verify that the caching mechanism correctly identifies and deduplicates identical
/// logical sub-expressions. This ensures that the Datalog program remains minimal.
///
/// # Input
/// - Logic: `(OR (AND P10 P11) (AND P10 P11))`
/// - Structure: An `OR` node with two identical `AND` children.
///
/// # Expected Output
/// - Only 2 unique auxiliary predicates should be created:
///   1. One for the `AND` (shared by both branches).
///   2. One for the root `OR`.
/// - The `rules` list must reflect this structural sharing.
#[test]
fn test_deduplication_cache() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Define terms and atoms for: (AND (P10 ?v0) (P11 ?v1))
    let v0 = builder.variable(0);
    let v1 = builder.variable(1);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let p11 = builder.atomic_formula_with_skeleton(11, vec![v1], 11);

    // Create two separate nodes representing the exact same logic.
    let and1 = builder.and(vec![p10, p11]);
    let and2 = builder.and(vec![p10, p11]);
    let root = builder.or(vec![and1, and2]);

    builder.set_root(root)?;
    let _head = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    // Industrial Requirement: Canonical Representation.
    // By merging identical sub-trees, we avoid redundant joins during the
    // saturation phase and keep the grounding graph as thin as possible.
    let unique_heads: std::collections::HashSet<_> = rules.iter().map(|r| r.head().skeleton_id()).collect();

    assert_eq!(
        unique_heads.len(),
        2,
        "The cache should have merged the identical AND expressions into a single auxiliary predicate"
    );

    Ok(())
}

/// # Objective
/// Verify that the encoder correctly extracts and preserves the distinct types of terms
/// (variables and constants) from an atomic formula.
///
/// # Input
/// - Logic: A single atomic formula `(P10 ?v0 c99)`.
/// - Terms: A variable `?v0` and a constant `c99`.
///
/// # Expected Output
/// - A successfully extracted atom with exactly 2 terms.
/// - The first term must be identified as a `Variable`.
/// - The second term must be identified as a `Constant`.
#[test]
fn test_mixed_terms_extraction() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Define a mixed set of terms: one action parameter and one fixed domain constant.
    let v0 = builder.variable(0);
    let c99 = builder.constant(99);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0, c99], 10);

    builder.set_root(p10)?;
    let result = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    // Industrial Requirement: Term Integrity.
    // The encoder must maintain the distinction between parameters and constants
    // to allow the Datalog solver to bind values correctly during saturation.
    let atom = result.expect("Should successfully extract the atomic formula");

    assert_eq!(atom.terms().len(), 2, "The atom must retain both terms");

    // Validate that the internal representation distinguishes between the term types.
    assert!(matches!(atom.terms()[0], Term::Variable(_)), "First term should be a Variable");
    assert!(matches!(atom.terms()[1], Term::Constant(_)), "Second term should be a Constant");

    Ok(())
}

/// # Objective
/// Verify the optimization of `AND` expressions with a single child. The encoder should
/// collapse the expression and return the child atom directly without generating
/// redundant auxiliary rules.
///
/// # Input
/// - Logic: `(AND (P10 ?v0))`
/// - Single child: Atomic formula `P10` with variable `?v0`.
///
/// # Expected Output
/// - `rules` list is empty (no unnecessary joining rules).
/// - The returned atom is exactly `P10` (Skeleton ID 10).
#[test]
fn test_and_optimization() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Construct: (AND (P10 ?v0))
    // Logically, (AND X) is equivalent to X.
    let v0 = builder.variable(0);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let root = builder.and(vec![p10]);

    builder.set_root(root)?;
    let result = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    // Industrial Requirement: Rule Minimization.
    // Reducing the number of predicates in the Datalog program directly
    // improves the performance of the fixpoint iteration (saturation).
    assert_eq!(
        rules.len(),
        0,
        "A single-child AND should not generate an auxiliary rule"
    );

    // Ensure the identity of the child is preserved.
    assert_eq!(
        result.unwrap().skeleton_id(),
        AtomSkeletonId::from(10),
        "The encoder must return the child atom directly for single-child AND nodes"
    );

    Ok(())
}

/// # Objective
/// Verify the optimization of `OR` expressions with a single child. The encoder should
/// skip auxiliary predicate generation and return the child atom directly.
///
/// # Input
/// - Logic: `(OR (P10 ?v0))`
/// - Single child: Atomic formula `P10` with variable `?v0`.
///
/// # Expected Output
/// - `rules` list is empty (no unnecessary auxiliary rules).
/// - The returned atom is exactly `P10` (Skeleton ID 10).
#[test]
fn test_or_optimization() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Construct: (OR (P10 ?v0))
    // A single-child OR is logically equivalent to the child itself.
    let v0 = builder.variable(0);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let root = builder.or(vec![p10]);

    builder.set_root(root)?;
    let result = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    // Industrial Requirement: Pass-through optimization.
    // Generating auxiliary predicates for single-child nodes increases the number
    // of joins during saturation without adding any logical value.
    assert_eq!(
        rules.len(),
        0,
        "A single-child OR should not generate an auxiliary rule"
    );

    // Verify that the returned atom is P10 directly.
    assert_eq!(
        result.unwrap().skeleton_id(),
        AtomSkeletonId::from(10),
        "The encoder must return the child atom directly for single-child OR nodes"
    );

    Ok(())
}

/// # Objective
/// Verify that an `OR` expression containing only ignored or unsupported operations
/// (like numeric assignments) results in no encoding output.
///
/// # Input
/// - Logic: `(OR (assign ?v0 1))`
/// - Note: The `assign` operation is typically not relevant for basic Boolean reachability.
///
/// # Expected Output
/// - `result` is `None`.
/// - `rules` list is empty.
#[test]
fn test_empty_or_ignored_logic() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Create an OR containing only ignored operations (e.g., numeric assignments).
    // In PDDL grounding, this often happens after filtering non-STRIPS features.
    let var = builder.variable(0);
    let cons = builder.constant(1);
    let n1 = builder.assign(var, cons);
    let root = builder.or(vec![n1]);

    builder.set_root(root)?;
    let result = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    // Industrial Requirement: Logic that doesn't contribute to reachability
    // constraints should return None to avoid polluting the Datalog engine.
    assert!(
        result.is_none(),
        "An OR logic with no valid Datalog facts should return None"
    );
    assert_eq!(
        rules.len(),
        0,
        "No auxiliary rules should be generated for empty or ignored OR branches"
    );

    Ok(())
}

/// # Objective
/// Verify that the encoder gracefully handles logical structures containing only
/// unsupported or ignored operations (e.g., numeric assignments).
///
/// # Input
/// - Logic: `(AND (assign ?v0 1))`
/// - Note: `assign` is a non-Boolean operation typically ignored in basic reachability.
///
/// # Expected Output
/// - `result` is `None` (no Datalog atom can represent this logic).
/// - `rules` list is empty (no auxiliary rules generated).
#[test]
fn test_ignored_and_logic() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Create an assignment operation: (assign ?v0 1)
    // Most Datalog encoders for reachability ignore numeric fluents.
    let var = builder.variable(0);
    let cons = builder.constant(1);
    let n1 = builder.assign(var, cons);
    let root = builder.and(vec![n1]);

    builder.set_root(root)?;
    let result = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    // Industrial Requirement: Clean failure/skipping.
    // The encoder should not panic or produce empty/broken rules when encountering ignored nodes.
    assert!(
        result.is_none(),
        "An AND containing only ignored nodes must return None"
    );
    assert_eq!(
        rules.len(),
        0,
        "No rules should be generated for purely ignored logic"
    );

    Ok(())
}

/// # Objective
/// Verify that the auxiliary predicate generation logic correctly deduplicates its
/// arguments. If multiple body atoms share the same variable, that variable should
/// only appear once in the auxiliary head's term list.
///
/// # Input
/// - Logic: `(AND (P10 ?v0) (P11 ?v0))`
/// - Shared variable: `?v0` (used in both atoms).
///
/// # Expected Output
/// - An auxiliary predicate head with arity 1.
/// - The head contains exactly one instance of `Variable(0)`.
#[test]
fn test_aux_predicate_arguments() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Construct: (AND (P10 ?v0) (P11 ?v0))
    // Both atoms reference the same variable ?v0.
    let v0 = builder.variable(0);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let p11 = builder.atomic_formula_with_skeleton(11, vec![v0], 11);
    let root = builder.and(vec![p10, p11]);

    builder.set_root(root)?;
    let result = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    let head = result.expect("Should return a head atom");

    // INDUSTRIAL REQUIREMENT: Minimal Arity.
    // If head.terms() contains 2 terms instead of 1, the Datalog engine will treat
    // it as a higher-dimensional relation, causing significant performance degradation.
    assert_eq!(
        head.terms().len(),
        1,
        "The auxiliary predicate must deduplicate its variables to maintain minimal arity"
    );

    Ok(())
}

/// # Objective
/// Verify that the encoder can process deeply nested logical structures without
/// stack overflow and correctly flattens or chains auxiliary rules.
///
/// # Input
/// - A chain of 19 nested `AND` operations: `(AND P1 (AND P2 (AND P3 ... P20)))`.
/// - All predicates share the same variable `?v0`.
///
/// # Expected Output
/// - A successful encoding (no panic/stack overflow).
/// - A non-empty set of Datalog rules representing the chain.
/// - The final head atom must be correctly linked to the bottom of the chain.
#[test]
fn test_deep_nesting() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Create a chain: (AND (P1) (AND (P2) (AND (P3) ...)))
    // This tests the recursion depth and rule chaining capability.
    let var = builder.variable(0);
    let mut last_node = builder.atomic_formula_with_skeleton(1, vec![var], 1);

    for i in 2..20 {
        let var = builder.variable(0);
        let pi = builder.atomic_formula_with_skeleton(i, vec![var], i);
        last_node = builder.and(vec![last_node, pi]);
    }

    builder.set_root(last_node)?;
    let result = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    // Industrial Requirement: Stability under depth.
    // Real-world domains can have massive logical expressions.
    assert!(result.is_some(), "Encoding failed to produce a head for a deep tree");

    // Ensure that rules were actually generated for the intermediate auxiliary predicates.
    assert!(rules.len() > 0, "No Datalog rules were generated for the nested structure");

    Ok(())
}

/// # Objective
/// Verify the completeness of variable projection in auxiliary predicates.
/// Every variable required by a sub-expression must be present in the generated
/// auxiliary head to allow correct data flow and unification.
///
/// # Input
/// - Logic: `(AND (P10 ?v0) (P11 ?v1))`
/// - Required variables: `?v0` and `?v1`
///
/// # Expected Output
/// - An auxiliary predicate head with arity 2.
/// - The head must contain both distinct variables `?v0` and `?v1`.
#[test]
fn test_variable_projection_completeness() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Define two distinct variables used in separate atoms within a conjunction.
    let v0 = builder.variable(0);
    let v1 = builder.variable(1);

    // Construct: (AND (P10 ?v0) (P11 ?v1))
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let p11 = builder.atomic_formula_with_skeleton(11, vec![v1], 11);
    let root = builder.and(vec![p10, p11]);

    builder.set_root(root)?;
    let result = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    let head = result.expect("Should return a head atom");
    let terms = head.terms();

    // Industrial Requirement: Completeness.
    // If ?v0 or ?v1 were missing, the resulting Datalog rules would be "unsafe"
    // or logically incomplete, leading to incorrect grounding results.
    assert_eq!(
        terms.len(),
        2,
        "The auxiliary predicate must project all variables required by its child atoms"
    );

    let has_v0 = terms.iter().any(|t| matches!(t, Term::Variable(id) if id.as_usize() == 0));
    let has_v1 = terms.iter().any(|t| matches!(t, Term::Variable(id) if id.as_usize() == 1));

    assert!(has_v0 && has_v1, "Both ?v0 and ?v1 must be present in the auxiliary head");

    Ok(())
}

/// # Objective
/// Validate the "Arity Reduction" optimization. Auxiliary predicates must only include
/// variables that appear within their specific logical scope, ignoring other action parameters.
///
/// # Input
/// - Action parameters: `[?v0, ?v1, ?v2, ?v3]`
/// - Logic: `(OR (P10 ?v0) (P11 ?v1))`
/// - Note: `?v2` and `?v3` are defined in the action but are absent from this sub-expression.
///
/// # Expected Output
/// - An auxiliary predicate head with arity 2 (containing only `?v0` and `?v1`).
/// - Variables `?v2` and `?v3` must be pruned to prevent combinatorial explosion.
#[test]
fn test_unused_parameter_reduction() -> TestResult {
    let (mut encoder, mut params) = setup_env();

    // Add an extra parameter (?v3) to the action context that is never used in the expression.
    params.push(TypedSymbol::new(VariableId::from(3), Type::root()));

    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Construct: (OR (P10 ?v0) (P11 ?v1))
    // ?v2 and ?v3 are "silent" parameters in this context.
    let v0 = builder.variable(0);
    let v1 = builder.variable(1);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let p11 = builder.atomic_formula_with_skeleton(11, vec![v1], 11);
    let root = builder.or(vec![p10, p11]);

    builder.set_root(root)?;
    let result = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    let head = result.expect("Should return a head atom");

    // INDUSTRIAL CHECK: The arity of the auxiliary predicate must be 2, not 4.
    // Reducing arity from N^4 to N^2 is what allows the grounding to scale.
    assert_eq!(
        head.terms().len(),
        2,
        "The auxiliary predicate must ignore action parameters not used in the sub-expression"
    );

    // Verify that the head contains the correct variables (?v0 and ?v1)
    let ids: Vec<_> = head.terms().iter().filter_map(|t| {
        if let Term::Variable(v) = t { Some(v.as_usize()) } else { None }
    }).collect();

    assert!(ids.contains(&0), "Head must contain variable ?v0");
    assert!(ids.contains(&1), "Head must contain variable ?v1");
    assert!(!ids.contains(&2), "Head must NOT contain unused variable ?v2");

    Ok(())
}

/// # Objective
/// Ensure the encoder correctly handles and preserves a mix of action parameters (variables)
/// and domain objects (constants) within an auxiliary predicate.
///
/// # Input
/// - Logic: (OR (P10 ?v0 c99))
/// - Variable: `?v0` (Action parameter)
/// - Constant: `c99` (Fixed domain object)
///
/// # Expected Output
/// - An auxiliary predicate head with arity 2.
/// - The head must contain both the variable `?v0` and the constant `c99`.
#[test]
fn test_constant_and_parameter_mix() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Define a variable from the action context and a fixed domain constant.
    let v0 = builder.variable(0);
    let c99 = builder.constant(99);

    // Construct: (OR (P10 ?v0 c99))
    // The OR forces the creation of an auxiliary predicate.
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0, c99], 10);
    let root = builder.or(vec![p10]);

    builder.set_root(root)?;
    let result = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    let head = result.expect("Should return a head atom");

    // Industrial Requirement: The auxiliary predicate must act as a transparent proxy.
    // It must not lose the constant nor the variable during the projection.
    assert_eq!(head.terms().len(), 2, "Auxiliary head must have 2 terms (1 variable + 1 constant)");

    let has_const = head.terms().iter().any(|t| matches!(t, Term::Constant(id) if id.as_usize() == 99));
    let has_var = head.terms().iter().any(|t| matches!(t, Term::Variable(id) if id.as_usize() == 0));

    assert!(has_const, "The constant 99 must be preserved in the auxiliary predicate terms");
    assert!(has_var, "The variable v0 must be preserved in the auxiliary predicate terms");

    Ok(())
}

/// # Objective
/// Verify that the caching mechanism correctly distinguishes between different logical
/// expressions and does not erroneously merge them into the same auxiliary predicate.
///
/// # Input
/// - Expression 1: (AND P10 P11)
/// - Expression 2: (AND P10 P12)
/// Note: Both share the atom P10 but differ in their second operand.
///
/// # Expected Output
/// - Two distinct auxiliary predicate IDs (Skeleton IDs).
/// - The cache must treat these as unique logical entities.
#[test]
fn test_cache_logic_identity() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();

    // --- Expression 1: (AND P10 P11) ---
    let mut b1 = ExprBuilder::new();
    let p10 = b1.atomic_formula_with_skeleton(10, vec![], 10);
    let p11 = b1.atomic_formula_with_skeleton(11, vec![], 11);
    let root1 = b1.and(vec![p10, p11]);
    b1.set_root(root1)?;
    let head_1 = encoder.encode_expr(&b1.finish(), &mut rules, &params)?;

    // --- Expression 2: (AND P10 P12) ---
    // Even if they share P10, the overall structure is different.
    let mut b2 = ExprBuilder::new();
    let p10_alt = b2.atomic_formula_with_skeleton(10, vec![], 10);
    let p12 = b2.atomic_formula_with_skeleton(12, vec![], 12);
    let root2 = b2.and(vec![p10_alt, p12]);
    b2.set_root(root2)?;
    let head_2 = encoder.encode_expr(&b2.finish(), &mut rules, &params)?;

    // Industrial Requirement: Auxiliary IDs must be unique to prevent logic corruption.
    assert_ne!(
        head_1.unwrap().skeleton_id(),
        head_2.unwrap().skeleton_id(),
        "The cache must not merge different logical expressions"
    );

    Ok(())
}

/// # Objective
/// Verify that the encoder correctly distinguishes between variables and constants
/// even if they share the same internal numeric ID.
///
/// # Input
/// - Logic: (AND (P10 ?v0) (P11 c0))
/// - Variables: `?v0` (ID: 0)
/// - Constants: `c0` (ID: 0)
///
/// # Expected Output
/// - The auxiliary predicate head must include the variable `?v0`.
/// - The internal term representation must differentiate `Variable(0)` from `Constant(0)`.
#[test]
fn test_variable_constant_separation() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Define ID 0 for both a variable and a constant to test disambiguation logic.
    // In many PDDL engines, raw IDs for different term types can overlap.
    let v0 = builder.variable(0);
    let c0 = builder.constant(0);

    // Construct: (AND (P10 ?v0) (P11 c0))
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let p11 = builder.atomic_formula_with_skeleton(11, vec![c0], 11);
    let root = builder.and(vec![p10, p11]);

    builder.set_root(root)?;
    let result = encoder.encode_expr(&builder.finish(), &mut rules, &params)?;

    let head = result.expect("Should return a head atom");
    let terms = head.terms();

    // Industrial requirement: The auxiliary predicate must carry the variable ?v0
    // so that the Datalog engine can perform the correct join/unification.
    let has_v0 = terms.iter().any(|t| matches!(t, Term::Variable(id) if id.as_usize() == 0));

    // Check if the variable is correctly identified despite the ID overlap with the constant.
    assert!(has_v0, "Variable v0 must be present in the auxiliary predicate head");

    Ok(())
}

/// # Objective
/// Verify that the cache distinguishes between symmetric and redundant logical structures.
///
/// # Input
/// - Case 1: (AND (P10 ?v0 ?v1) (P10 ?v1 ?v0)) -> Symmetric relationship.
/// - Case 2: (AND (P10 ?v0 ?v1) (P10 ?v0 ?v1)) -> Simple redundancy.
///
/// # Expected Output
/// - The skeleton IDs for both cases must be different.
#[test]
fn test_logic_symmetry_breaking() -> TestResult {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();

    // --- Case 1: Symmetric (at ?a ?b) AND (at ?b ?a) ---
    let mut b1 = ExprBuilder::new();
    let v0 = b1.variable(0);
    let v1 = b1.variable(1);
    let p1 = b1.atomic_formula_with_skeleton(10, vec![v0, v1], 10);
    let p2 = b1.atomic_formula_with_skeleton(10, vec![v1, v0], 10);
    let and = b1.and(vec![p1, p2]);
    b1.set_root(and)?;
    let head1 = encoder.encode_expr(&b1.finish(), &mut rules, &params)?;

    // --- Case 2: Redundant (at ?a ?b) AND (at ?a ?b) ---
    let mut b2 = ExprBuilder::new();
    let v0_b2 = b2.variable(0);
    let v1_b2 = b2.variable(1);
    let p3 = b2.atomic_formula_with_skeleton(10, vec![v0_b2, v1_b2], 10);
    let p4 = b2.atomic_formula_with_skeleton(10, vec![v0_b2, v1_b2], 10);
    let and  = b2.and(vec![p3, p4]);
    b2.set_root(and)?;
    let head2 = encoder.encode_expr(&b2.finish(), &mut rules, &params)?;

    assert_ne!(
        head1.unwrap().skeleton_id(),
        head2.unwrap().skeleton_id(),
        "The cache must not conflate symmetric relations with redundant ones"
    );

    Ok(())
}
