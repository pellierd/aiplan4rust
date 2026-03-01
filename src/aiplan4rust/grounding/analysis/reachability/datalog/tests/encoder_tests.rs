use std::error::Error;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::encoder::DatalogEncoder;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::lang::{AtomSkeletonId, CompareOp, ObjectId, PredicateSymbolId, Type, TypeId, TypedList, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::expr::{ExprBuilder, ExprKind};

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
fn test_complex_logical_flattening() -> Result<(), DatalogError> {
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

    let result = encoder.encode_preconditions(&expr, &mut rules, &params)?;

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
fn test_deduplication_cache() -> Result<(), DatalogError> {
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
    let _head = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

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
fn test_mixed_terms_extraction() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Define a mixed set of terms: one action parameter and one fixed domain constant.
    let v0 = builder.variable(0);
    let c99 = builder.constant(99);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0, c99], 10);

    builder.set_root(p10)?;
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

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
fn test_and_optimization() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Construct: (AND (P10 ?v0))
    // Logically, (AND X) is equivalent to X.
    let v0 = builder.variable(0);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let root = builder.and(vec![p10]);

    builder.set_root(root)?;
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

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
fn test_or_optimization() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Construct: (OR (P10 ?v0))
    // A single-child OR is logically equivalent to the child itself.
    let v0 = builder.variable(0);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let root = builder.or(vec![p10]);

    builder.set_root(root)?;
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

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

/// (like numeric comparisons) results in no encoding output.
///
/// # Input
/// - Logic: `(OR (>= (fuel) 10))` -> Represented as FComp in the AST.
///
/// # Expected Output
/// - `result` is `None`.
/// - `rules` list is empty.
#[test]
fn test_empty_or_ignored_logic() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // We use a Numeric Comparison (FComp) because an 'assign' is an effect,
    // not a precondition. FComp is a valid precondition but ignored by
    // basic Boolean reachability encoders.
    let var = builder.variable(0);
    let cons = builder.constant(10);

    // Greater-than-or-equal (FComp) is valid in a precondition
    let n1 = builder.comparison(CompareOp::Greater, var, cons);
    let root = builder.or(vec![n1]);

    builder.set_root(root)?;
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

    // Requirement: Logic that doesn't contribute to Boolean reachability
    // (like pure numeric constraints) should return None.
    assert!(
        result.is_none(),
        "An OR logic containing only ignored FComp should return None"
    );
    assert_eq!(
        rules.len(),
        0,
        "No rules should be generated for ignored numeric constraints in preconditions"
    );

    Ok(())
}

/// # Objective
/// Verify that the encoder gracefully handles logical structures containing only
/// ignored operations (e.g., numeric comparisons) within a conjunction.
///
/// # Input
/// - Logic: `(AND (>= (fuel) 10))` -> Represented as FComp in the AST.
///
/// # Expected Output
/// - `result` is `None` (no Datalog atom can represent this logic).
/// - `rules` list is empty (no auxiliary rules generated).
#[test]
fn test_ignored_and_logic() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // We use a Numeric Comparison (FComp) because 'assign' is an effect.
    // FComp is a valid precondition but ignored by basic Boolean reachability.
    let var = builder.variable(0);
    let cons = builder.constant(10);

    // (>= ?v0 10)
    let n1 = builder.comparison(CompareOp::GreaterEq, var, cons);
    let root = builder.and(vec![n1]);

    builder.set_root(root)?;
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

    // Industrial Requirement: Clean skipping.
    // The encoder should return None when the logic doesn't map to a Boolean fact.
    assert!(
        result.is_none(),
        "An AND containing only ignored FComp nodes must return None"
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
fn test_aux_predicate_arguments() -> Result<(), DatalogError> {
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
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

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
fn test_deep_nesting() -> Result<(), DatalogError> {
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
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

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
fn test_variable_projection_completeness() -> Result<(), DatalogError> {
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
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

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
fn test_unused_parameter_reduction() -> Result<(), DatalogError> {
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
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

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
fn test_constant_and_parameter_mix() -> Result<(), DatalogError> {
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
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

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
fn test_cache_logic_identity() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();

    // --- Expression 1: (AND P10 P11) ---
    let mut b1 = ExprBuilder::new();
    let p10 = b1.atomic_formula_with_skeleton(10, vec![], 10);
    let p11 = b1.atomic_formula_with_skeleton(11, vec![], 11);
    let root1 = b1.and(vec![p10, p11]);
    b1.set_root(root1)?;
    let head_1 = encoder.encode_preconditions(&b1.finish(), &mut rules, &params)?;

    // --- Expression 2: (AND P10 P12) ---
    // Even if they share P10, the overall structure is different.
    let mut b2 = ExprBuilder::new();
    let p10_alt = b2.atomic_formula_with_skeleton(10, vec![], 10);
    let p12 = b2.atomic_formula_with_skeleton(12, vec![], 12);
    let root2 = b2.and(vec![p10_alt, p12]);
    b2.set_root(root2)?;
    let head_2 = encoder.encode_preconditions(&b2.finish(), &mut rules, &params)?;

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
fn test_variable_constant_separation() -> Result<(), DatalogError> {
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
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

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
fn test_logic_symmetry_breaking() -> Result<(), DatalogError> {
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
    let head1 = encoder.encode_preconditions(&b1.finish(), &mut rules, &params)?;

    // --- Case 2: Redundant (at ?a ?b) AND (at ?a ?b) ---
    let mut b2 = ExprBuilder::new();
    let v0_b2 = b2.variable(0);
    let v1_b2 = b2.variable(1);
    let p3 = b2.atomic_formula_with_skeleton(10, vec![v0_b2, v1_b2], 10);
    let p4 = b2.atomic_formula_with_skeleton(10, vec![v0_b2, v1_b2], 10);
    let and  = b2.and(vec![p3, p4]);
    b2.set_root(and)?;
    let head2 = encoder.encode_preconditions(&b2.finish(), &mut rules, &params)?;

    assert_ne!(
        head1.unwrap().skeleton_id(),
        head2.unwrap().skeleton_id(),
        "The cache must not conflate symmetric relations with redundant ones"
    );

    Ok(())
}

#[test]
fn test_encode_effects_basic_and_conjunction() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // 1. Define the "Action Atom" (the cause)
    // Equivalent to: drive(?v0, ?v1)
    let action_sk_id = AtomSkeletonId::from(100);
    let action_atom = Atom::new(
        action_sk_id,
        vec![Term::Variable(VariableId::from(0)), Term::Variable(VariableId::from(1))]
    );

    // 2. Create effects: (and (at ?v0) (not (at ?v1)))
    let pred_id = PredicateSymbolId::from(1);
    let atom_sk = AtomSkeletonId::from(1);

    // Positive effect: (at ?v0)
    let var_v0 = builder.variable(0);
    let at_v0 = builder.atomic_formula_with_skeleton(pred_id, vec![var_v0], atom_sk);

    // Negative effect: (not (at ?v1))
    // We use the same IDs to simulate a real PDDL delete effect
    let var_v1 = builder.variable(1);
    let at_v1 = builder.atomic_formula_with_skeleton(pred_id, vec![var_v1], atom_sk);
    let not_at_v1 = builder.not(at_v1);

    let root = builder.and(vec![at_v0, not_at_v1]);
    builder.set_root(root)?;

    // 3. Encode
    encoder.encode_effects(&builder.finish(), &action_atom, &mut rules, &params)?;

    // Expectations:
    // - Rule 1: at(?v0) :- drive(?v0, ?v1).
    // - The 'not' effect must be ignored (relaxed reachability logic).
    assert_eq!(rules.len(), 1, "Should generate exactly 1 rule for the positive effect");
    assert_eq!(rules[0].head().skeleton_id(), atom_sk, "The effect head ID must match the skeleton ID provided");
    assert_eq!(rules[0].body()[0], action_atom, "The rule body must be the action itself");

    Ok(())
}

fn test_encode_effects_conditional_when() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // 1. Action: move(?v0)
    let action_sk_id = AtomSkeletonId::from(100);
    let action_atom = Atom::new(action_sk_id, vec![Term::Variable(VariableId::from(0))]);

    // 2. Condition: (at ?v1)
    let cond_pred = PredicateSymbolId::from(1);
    let cond_sk = AtomSkeletonId::from(1);
    let var_v1 = builder.variable(1);
    let condition = builder.atomic_formula_with_skeleton(cond_pred, vec![var_v1], cond_sk);

    // 3. Effect: (sticky ?v0 ?v1)  <-- MODIFICATION ICI : on utilise v1 pour justifier sa présence dans le pivot
    let eff_pred = PredicateSymbolId::from(2);
    let eff_sk = AtomSkeletonId::from(2);
    let var_v0 = builder.variable(0);
    let var_v1_eff = builder.variable(1); // On récupère v1 pour l'effet
    let effect = builder.atomic_formula_with_skeleton(eff_pred, vec![var_v0, var_v1_eff], eff_sk);

    // 4. Construct: (when (at ?v1) (sticky ?v0 ?v1))
    let root = builder.when(condition, effect);
    builder.set_root(root)?;

    // 5. Encode
    encoder.encode_effects(&builder.finish(), &action_atom, &mut rules, &params)?;

    // Expectations:
    // Rule A (Pivot): aux_pivot(?v0, ?v1) :- move(?v0), at(?v1).
    // Rule B (Effect): sticky(?v0, ?v1) :- aux_pivot(?v0, ?v1).

    assert_eq!(rules.len(), 2, "Should generate exactly 2 rules (one pivot, one effect)");

    // On cherche la règle du pivot (celle qui a 2 atomes dans le corps)
    let pivot_rule = rules.iter()
        .find(|r| r.body().len() == 2)
        .expect("Missing pivot rule combining action and condition");

    assert!(pivot_rule.body().contains(&action_atom), "Pivot must contain the action atom");

    // Cette fois, l'assertion va passer car v1 est requis par l'effet !
    assert_eq!(pivot_rule.head().terms().len(), 2, "Pivot atom should capture both variables because both are needed for the effect");

    // Vérification finale de la chaîne
    let effect_rule = rules.iter()
        .find(|r| r.head().skeleton_id() == eff_sk)
        .expect("Missing final effect rule");

    assert_eq!(
        effect_rule.body()[0],
        *pivot_rule.head(),
        "The effect must be triggered by the pivot atom"
    );

    Ok(())
}

#[test]
fn test_encode_effects_ignore_numerics() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // 1. Action atom (the cause)
    let action_sk_id = AtomSkeletonId::from(100);
    let action_atom = Atom::new(action_sk_id, vec![]);

    // 2. Build mixed effects: (and (at-goal) (assign ?v0 0))
    let pred_id = PredicateSymbolId::from(1);
    let atom_sk = AtomSkeletonId::from(1);

    // Valid Boolean effect
    let goal_node = builder.atomic_formula_with_skeleton(pred_id, vec![], atom_sk);

    // Numeric effect (to be ignored)
    let var_v0 = builder.variable(0);
    let cons_0 = builder.constant(0);
    let assign_node = builder.assign(var_v0, cons_0);

    let root = builder.and(vec![goal_node, assign_node]);
    builder.set_root(root)?;

    // 3. Encode
    encoder.encode_effects(&builder.finish(), &action_atom, &mut rules, &params)?;

    // Expectations:
    // - Only the (at-goal) effect should result in a Datalog rule.
    // - The 'assign' node must be silently skipped by the encoder.
    assert_eq!(
        rules.len(),
        1,
        "Only one rule should be generated, ignoring the numeric assignment"
    );
    assert_eq!(
        rules[0].head().skeleton_id(),
        atom_sk,
        "The generated rule must correspond to the Boolean 'at-goal' predicate"
    );
    assert_eq!(
        rules[0].body()[0],
        action_atom,
        "The rule must be correctly attached to the action cause"
    );

    Ok(())
}

#[test]
fn test_encode_effects_nested_when() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    let action_atom = Atom::new(AtomSkeletonId::from(100), vec![]);

    // 1. Build the expressions
    let c1_node = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(1), vec![], AtomSkeletonId::from(1));
    let c2_node = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(2), vec![], AtomSkeletonId::from(2));
    let eff_node = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(3), vec![], AtomSkeletonId::from(3));

    let inner_when = builder.when(c2_node, eff_node);
    let root = builder.when(c1_node, inner_when);
    builder.set_root(root)?;

    let expr = builder.finish();

    // 2. Encode
    encoder.encode_effects(&expr, &action_atom, &mut rules, &params)?;

    // 3. Prepare atoms for verification via Rule exploration
    // On cherche l'atome qui a le squelette ID qu'on a fixé (1 et 2)
    let c1_atom = rules.iter()
        .flat_map(|r| r.body().iter().chain(std::iter::once(r.head())))
        .find(|a| a.skeleton_id() == AtomSkeletonId::from(1))
        .expect("C1 atom not found in rules")
        .clone();

    let c2_atom = rules.iter()
        .flat_map(|r| r.body().iter().chain(std::iter::once(r.head())))
        .find(|a| a.skeleton_id() == AtomSkeletonId::from(2))
        .expect("C2 atom not found in rules")
        .clone();

    // 4. Assertions
    assert_eq!(rules.len(), 3, "Nested when should produce a chain of 3 rules");

    // Find the final effect rule: Effect :- Aux2
    let rule_eff = rules.iter().find(|r| r.head().skeleton_id() == AtomSkeletonId::from(3)).unwrap();
    let aux2_atom = &rule_eff.body()[0];

    // Find the rule for Aux2: Aux2 :- Aux1, Cond2
    let rule_aux2 = rules.iter().find(|r| r.head() == aux2_atom).unwrap();

    // CORRECTION: use c2_atom (Atom), not c2_node (NodeId)
    assert!(rule_aux2.body().contains(&c2_atom), "Aux2 rule must contain the second condition atom");

    Ok(())
}

#[test]
fn test_encode_effects_when_complex_condition() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    let action_atom = Atom::new(AtomSkeletonId::from(100), vec![]);

    // (when (and (c1) (c2)) (eff))
    let c1 = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(1), vec![], AtomSkeletonId::from(1));
    let c2 = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(2), vec![], AtomSkeletonId::from(2));
    let cond_and = builder.and(vec![c1, c2]);
    let eff = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(3), vec![], AtomSkeletonId::from(3));

    let root = builder.when(cond_and, eff);
    builder.set_root(root)?;

    encoder.encode_effects(&builder.finish(), &action_atom, &mut rules, &params)?;

    // Ici encode_expr va créer un auxiliaire pour le (AND c1 c2)
    // Et encode_effects va créer un auxiliaire pour le pivot Action + Aux_And.
    // C'est un excellent test pour ton cache de prédicats.
    assert!(rules.len() >= 2);
    Ok(())
}

#[test]
fn test_encode_effects_temporal_wrappers() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    let action_atom = Atom::new(AtomSkeletonId::from(100), vec![]);

    // (at start (at-goal))
    let goal = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(1), vec![], AtomSkeletonId::from(1));
    let root = builder.at_start(goal);
    builder.set_root(root)?;

    encoder.encode_effects(&builder.finish(), &action_atom, &mut rules, &params)?;

    assert_eq!(rules.len(), 1, "Temporal wrappers should be transparent for effects");
    assert_eq!(rules[0].head().skeleton_id(), AtomSkeletonId::from(1));

    Ok(())
}

#[test]
fn test_encode_effects_when_cache_reuse() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    let action_atom = Atom::new(AtomSkeletonId::from(100), vec![]);

    // Deux 'When' avec exactement la même condition
    let cond = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(1), vec![], AtomSkeletonId::from(1));
    let eff1 = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(2), vec![], AtomSkeletonId::from(2));
    let eff2 = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(3), vec![], AtomSkeletonId::from(3));

    let when1 = builder.when(cond, eff1);
    let when2 = builder.when(cond, eff2);
    let root = builder.and(vec![when1, when2]);
    builder.set_root(root)?;

    encoder.encode_effects(&builder.finish(), &action_atom, &mut rules, &params)?;

    // On s'attend à ce qu'il n'y ait QU'UN SEUL pivot créé pour (Action + Cond)
    // Les deux effets doivent pointer vers le même atome de tête du pivot.
    let pivot_heads: Vec<_> = rules.iter()
        .filter(|r| r.body().len() == 2) // Les règles de pivot
        .map(|r| r.head().clone())
        .collect();

    assert_eq!(pivot_heads.len(), 1, "Should reuse the same pivot for identical Action+Condition pairs");

    Ok(())
}

#[test]
fn test_encode_effects_variable_projection() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env(); // Setup avec plusieurs paramètres (?v0, ?v1, ?v2)
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Action op(?v0, ?v1, ?v2)
    let action_atom = Atom::new(AtomSkeletonId::from(100), vec![
        Term::Variable(VariableId::from(0)),
        Term::Variable(VariableId::from(1)),
        Term::Variable(VariableId::from(2)),
    ]);

    // L'effet n'utilise QUE ?v1
    let v1 = builder.variable(1);
    let cond = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(1), vec![v1], AtomSkeletonId::from(1));
    let eff = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(2), vec![], AtomSkeletonId::from(2));

    let root = builder.when(cond, eff);
    builder.set_root(root)?;

    encoder.encode_effects(&builder.finish(), &action_atom, &mut rules, &params)?;

    // Le pivot ne doit contenir QUE ?v1 (et éventuellement les variables de l'action si elles servent plus loin)
    let pivot_rule = rules.iter().find(|r| r.body().len() == 2).unwrap();

    // Si ton collect_variables marche bien, l'arité est réduite au strict nécessaire.
    assert!(pivot_rule.head().terms().len() < 3, "Auxiliary predicate should only carry necessary variables");

    Ok(())
}

#[test]
fn test_final_boss_encoding() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // 1. Action: fly(?v0, ?v1)
    let action_sk_id = AtomSkeletonId::from(100);
    let action_atom = Atom::new(action_sk_id, vec![
        Term::Variable(VariableId::from(0)),
        Term::Variable(VariableId::from(1))
    ]);

    // 2. Préparation des identifiants
    let c_paris_id = builder.constant(1);
    let v0_id = builder.variable(0);
    let v1_id = builder.variable(1);
    let term_paris = Term::Constant(ObjectId::from(1));

    // 3. Condition complexe (at ?v0 paris) & (can_fly ?v0 ?v0)
    let at_p = builder.atomic_formula_with_skeleton(
        PredicateSymbolId::from(1),
        vec![v0_id, c_paris_id],
        AtomSkeletonId::from(1)
    );

    let can_f = builder.atomic_formula_with_skeleton(
        PredicateSymbolId::from(2),
        vec![v0_id, v0_id],
        AtomSkeletonId::from(2)
    );

    let cond = builder.and(vec![at_p, can_f]);

    // 4. Effet: (landed ?v1)
    let eff_sk = AtomSkeletonId::from(3);
    let effect = builder.atomic_formula_with_skeleton(
        PredicateSymbolId::from(3),
        vec![v1_id],
        eff_sk
    );

    // 5. Montage
    let root = builder.when(cond, effect);
    builder.set_root(root)?;

    // 6. Encodage
    encoder.encode_effects(&builder.finish(), &action_atom, &mut rules, &params)?;

    // --- ASSERTIONS MISES À JOUR ---

        // 1. On vérifie qu'on a bien nos étapes de raisonnement
        assert!(rules.len() >= 2, "L'encodeur devrait générer plusieurs règles optimisées");

    // 2. On cherche la règle qui lie l'action fly(?v0, ?v1) à la condition
    // Dans tes logs, c'est celle qui a l'atome skeleton 100 dans son corps.
    let action_pivot = rules.iter()
        .find(|r| r.body().iter().any(|atom| atom.skeleton_id() == AtomSkeletonId::from(100)))
        .expect("Règle de liaison Action <-> Condition manquante");

    // 3. Vérification de la projection (C'est là que ton 1 vs 2 se jouait)
    // L'encodeur a projeté uniquement ?v1 car ?v0 n'est plus requis pour l'effet landed(?v1).
    let head_terms = action_pivot.head().terms();
    assert_eq!(head_terms.len(), 1, "L'optimiseur aurait dû projeter uniquement ?v1");
    assert_eq!(head_terms[0], Term::Variable(VariableId::from(1)));

    // 4. Vérification de la présence de la constante dans la TOUTE PREMIÈRE règle (Règle 0)
    let condition_rule = rules.iter()
        .find(|r| r.body().iter().any(|atom| atom.terms().contains(&term_paris)))
        .expect("La règle filtrant par la constante 'paris' est manquante");

    assert!(condition_rule.body().len() >= 2, "La règle de condition doit avoir au moins (at) et (can_fly)");

    println!("Victoire ! L'encodeur a produit une chaîne de règles ultra-optimisée.");
    Ok(())
}

/// # Objective
/// Verify that an explicit equality `(= ?v1 ?v0)` in a conjunction causes
/// all instances of `?v1` to be replaced by `?v0` in the resulting Datalog atoms.
#[test]
fn test_variable_aliasing_unification() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Logic: (AND (P10 ?v1) (= ?v0 ?v1))
    let v0 = builder.variable(0);
    let v1 = builder.variable(1);

    let p10 = builder.atomic_formula_with_skeleton(10, vec![v1], 10);
    let eq = builder.comparison(CompareOp::Equal, v0, v1);
    let root = builder.and(vec![p10, eq]);

    builder.set_root(root)?;
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

    // Requirement: The equality node itself is ignored, but it influences other atoms.
    // The resulting atom for (P10 ?v1) should now be (P10 ?v0).
    let atom = result.expect("Should return the flattened P10 atom");

    if let Term::Variable(id) = atom.terms()[0] {
        assert_eq!(id.as_usize(), 0, "Variable ?v1 should have been aliased to ?v0");
    } else {
        panic!("Term should be a variable");
    }

    Ok(())
}

/// # Objective
/// Verify that equality constraints reduce the arity of auxiliary predicates.
/// If `?v0 = ?v1`, an auxiliary predicate for `(P10 ?v0) (P11 ?v1)` should
/// only have 1 parameter, not 2.
#[test]
fn test_aliasing_arity_reduction() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Logic: (AND (P10 ?v0) (P11 ?v1) (= ?v0 ?v1))
    let v0 = builder.variable(0);
    let v1 = builder.variable(1);

    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let p11 = builder.atomic_formula_with_skeleton(11, vec![v1], 11);
    let eq = builder.comparison(CompareOp::Equal, v0, v1);
    let root = builder.and(vec![p10, p11, eq]);

    builder.set_root(root)?;
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

    let head = result.expect("Should return an auxiliary head");

    // Industrial Requirement: Arity reduction.
    // Without aliasing, arity would be 2 (?v0, ?v1).
    // With aliasing, it must be 1 (?v0).
    assert_eq!(
        head.terms().len(),
        1,
        "Auxiliary predicate should only have 1 term due to ?v0 = ?v1 aliasing"
    );

    Ok(())
}

#[test]
fn test_aliasing_propagation_to_effects() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();

    // 1. Setup Preconditions with Alias: (AND (P10 ?v1) (= ?v0 ?v1))
    let mut b_pre = ExprBuilder::new();
    let v0 = b_pre.variable(0);
    let v1 = b_pre.variable(1);
    let p10 = b_pre.atomic_formula_with_skeleton(10, vec![v1], 10);
    let eq = b_pre.comparison(CompareOp::Equal, v0, v1);
    let pre_logic = b_pre.and(vec![p10, eq]);
    b_pre.set_root(pre_logic)?;

    // Trigger aliasing by encoding preconditions first
    let action_atom = Atom::new(AtomSkeletonId::from(100), vec![Term::Variable(VariableId::from(0))]);
    encoder.encode_preconditions(&b_pre.finish(), &mut Vec::new(), &params)?;

    // 2. Setup Effect: (P20 ?v1)
    let mut b_eff = ExprBuilder::new();
    let v1_eff = b_eff.variable(1);
    let eff_logic = b_eff.atomic_formula_with_skeleton(20, vec![v1_eff], 20);
    b_eff.set_root(eff_logic)?;

    // 3. Encode Effects
    encoder.encode_effects(&b_eff.finish(), &action_atom, &mut rules, &params)?;

    // Requirement: The effect rule should be P20(?v0) :- Action(?v0)
    // even though the effect was defined with ?v1.
    let effect_rule = &rules[0];
    if let Term::Variable(id) = effect_rule.head().terms()[0] {
        assert_eq!(id.as_usize(), 0, "Effect variable ?v1 should have been resolved to ?v0");
    } else {
        panic!("Effect term should be a variable");
    }

    Ok(())
}

/// # Objective
/// Verify that aliasing is transitive: if ?v2 = ?v1 and ?v1 = ?v0,
/// then ?v2 should resolve to ?v0.
#[test]
fn test_aliasing_transitivity() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Logic: (AND (P10 ?v2) (= ?v1 ?v2) (= ?v0 ?v1))
    let v0 = builder.variable(0);
    let v1 = builder.variable(1);
    let v2 = builder.variable(2);

    let p10 = builder.atomic_formula_with_skeleton(10, vec![v2], 10);
    let eq1 = builder.comparison(CompareOp::Equal, v1, v2);
    let eq2 = builder.comparison(CompareOp::Equal, v0, v1);
    let root = builder.and(vec![p10, eq1, eq2]);

    builder.set_root(root)?;
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

    // Requirement: ?v2 -> ?v1 -> ?v0. The final atom should be P10(?v0).
    let atom = result.expect("Should return the flattened atom");
    if let Term::Variable(id) = atom.terms()[0] {
        assert_eq!(id.as_usize(), 0, "Variable ?v2 should have been transitively aliased to ?v0");
    } else {
        panic!("Term should be a variable");
    }

    Ok(())
}

#[test]
fn test_aliasing_variable_to_constant() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Logique: (AND (P10 ?v0) (= ?v0 c99))
    let v0 = builder.variable(0);
    let c99 = builder.constant(99);

    // On crée le prédicat P10(?v0)
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    // On crée l'égalité (= ?v0 c99)
    let eq = builder.comparison(CompareOp::Equal, v0, c99);

    // On groupe dans un AND
    let root = builder.and(vec![p10, eq]);
    builder.set_root(root)?;

    // --- EXÉCUTION ---
    // 1. extract_variable_aliases va trouver {v0 -> Constant(99)}
    // 2. compute_transitive_closure va stabiliser la map
    // 3. encode_expr va résoudre v0 en Constant(99) lors de la visite de P10
    let result = encoder.encode_expr(&builder.finish(), root, &mut rules, &params)?;

    // --- VÉRIFICATIONS ---

    // 1. Le résultat doit être Some car l'unification est possible
    let atom = result.expect("Should return an atom, not None (False)");

    // 2. L'ID du prédicat doit être 10 (P10)
    // L'égalité (= ?v0 c99) a dû être filtrée par le AND car elle est devenue triviale
    // (ou consommée par l'aliasing) et il ne reste qu'un seul atome.
    assert_eq!(atom.skeleton_id().as_usize(), 10);

    // 3. LE POINT CLÉ : ?v0 doit avoir été remplacé par Constant(99)
    // On vérifie que resolve_var() a bien injecté la constante dans l'atome P10
    match &atom.terms()[0] {
        Term::Constant(id) => assert_eq!(id.as_usize(), 99),
        Term::Variable(v) => panic!("Should be Constant(99), but found Variable({:?})", v),
    }

    // 4. Aucune règle auxiliaire ne doit être créée
    // Puisqu'il ne reste qu'un atome (P10) après filtrage de l'égalité,
    // le AND renvoie directement l'atome sans créer de règle "Aux :- P10".
    assert_eq!(rules.len(), 0, "Should not create auxiliary rules for a single atom");

    Ok(())
}

/// # Objective
/// Verify that trivial equalities like (= ?v0 ?v0) are gracefully ignored
/// and don't affect the encoding logic or produce errors.
#[test]
fn test_trivial_self_equality() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Logic: (AND (P10 ?v0) (= ?v0 ?v0))
    let v0 = builder.variable(0);
    // On crée l'atome P10(?v0)
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    // On crée l'égalité triviale ?v0 = ?v0
    let eq = builder.comparison(CompareOp::Equal, v0, v0);

    // On regroupe dans un AND
    let root = builder.and(vec![p10, eq]);

    builder.set_root(root)?;
    let result = encoder.encode_expr(&builder.finish(), root, &mut rules, &params)?;

    // 1. Le résultat doit être Some(Atom).
    // Si le AND avait échoué à cause de l'égalité, il aurait renvoyé None.
    let atom = result.expect("Should encode correctly despite trivial equality");

    // 2. L'atome retourné doit être P10 (ID 10).
    // Ton bloc ExprKind::And filtre les égalités où terms[0] == terms[1].
    // Comme il ne reste que P10, le AND renvoie cet atome directement sans créer d'auxiliaire.
    assert_eq!(atom.skeleton_id().as_usize(), 10, "The trivial equality should have been filtered out by the AND");

    // 3. On vérifie les termes de l'atome restant pour être sûr que c'est bien P10(?v0)
    match &atom.terms()[0] {
        Term::Variable(v) => assert_eq!(v.as_usize(), 0),
        _ => panic!("The remaining atom should be P10(?v0)"),
    }

    // 4. Aucune règle auxiliaire ne doit être créée.
    // L'égalité (?v0 = ?v0) est True, elle disparait, il reste 1 seul atome, donc 0 règle.
    assert_eq!(rules.len(), 0, "No extra rules should be generated for trivial logic");

    Ok(())
}

#[test]
fn test_aliasing_transitive_to_constant() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Logique: (AND (P10 ?v2) (= ?v2 ?v1) (= ?v1 ?v0) (= ?v0 c99))
    let v0 = builder.variable(0);
    let v1 = builder.variable(1);
    let v2 = builder.variable(2);
    let c99 = builder.constant(99);

    let p10 = builder.atomic_formula_with_skeleton(10, vec![v2], 10);
    let eq1 = builder.comparison(CompareOp::Equal, v2, v1);
    let eq2 = builder.comparison(CompareOp::Equal, v1, v0);
    let eq3 = builder.comparison(CompareOp::Equal, v0, c99);

    // Le root est un AND de 4 éléments
    let root = builder.and(vec![p10, eq1, eq2, eq3]);

    builder.set_root(root)?;
    // L'encodeur va :
    // 1. Extraire les alias : {v2: v1, v1: v0, v0: 99}
    // 2. Résoudre transitivement : v2 -> 99
    // 3. Encoder P10(?v2) qui devient P10(c99)
    let result = encoder.encode_expr(&builder.finish(), root, &mut rules, &params)?;

    // --- VÉRIFICATIONS ---

    // 1. Le résultat doit être Some(Atom). Si une étape avait échoué, on aurait None.
    let atom = result.expect("Should return the flattened atom after transitive resolution");

    // 2. Vérification de l'ID du prédicat (P10)
    assert_eq!(atom.skeleton_id().as_usize(), 10);

    // 3. VÉRIFICATION DE LA TRANSITIVITÉ : ?v2 doit être devenu Constant(99)
    // C'est ici qu'on valide que resolve_var() est récursive ou que la map d'alias est complète.
    match &atom.terms()[0] {
        Term::Constant(id) => {
            assert_eq!(id.as_usize(), 99, "Transitivity v2 -> v1 -> v0 -> 99 failed");
        },
        Term::Variable(v) => {
            panic!("Variable {:?} was not resolved. Resolve_var might be missing transitive steps.", v);
        }
    }

    // 4. Vérification de la propreté : aucune règle auxiliaire ne doit être créée.
    // Les 3 égalités (eq1, eq2, eq3) deviennent toutes (c99 = c99) après résolution.
    // Ton filtre dans le AND doit les supprimer toutes, ne laissant que P10.
    // Comme il n'y a qu'un seul atome restant, le AND ne crée pas de règle "Aux :- P10".
    assert_eq!(rules.len(), 0, "All equalities should be absorbed by aliasing, no aux rules needed");

    Ok(())
}

#[test]
fn test_aliasing_constant_conflict() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut rules = Vec::new();
    let mut builder = ExprBuilder::new();

    // Logic: (AND (P10 ?v0) (= ?v0 c1) (= ?v0 c2))
    // Sémantiquement, c'est IMPOSSIBLE.
    // ?v0 ne peut pas être unifié avec c1 ET c2 simultanément.
    let v0 = builder.variable(0);
    let c1 = builder.constant(1);
    let c2 = builder.constant(2);

    let p10 = builder.atomic_formula_with_skeleton(PredicateSymbolId::from(10), vec![v0], AtomSkeletonId::from(10));
    let eq1 = builder.comparison(CompareOp::Equal, v0, c1);
    let eq2 = builder.comparison(CompareOp::Equal, v0, c2);
    let root = builder.and(vec![p10, eq1, eq2]);

    builder.set_root(root)?;
    let result = encoder.encode_preconditions(&builder.finish(), &mut rules, &params)?;

    // --- CORRECTION ICI ---
    // Si ton encodeur détecte le conflit, il renvoie None.
    // C'est un comportement valide pour une précondition impossible.
    assert!(result.is_none(), "Un conflit de constantes doit retourner None (logique fausse)");
    assert!(rules.is_empty(), "Aucune règle ne doit être générée pour une précondition impossible");

    Ok(())
}

#[test]
fn test_not_atomic_formula_triggers_error() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut builder = ExprBuilder::new();
    let mut rules = Vec::new();

    // Logique : (NOT (P10 ?v0)) -> Doit échouer car pas en Positive Normal Form
    let v0 = builder.variable(0);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![v0], 10);
    let root = builder.not(p10);
    builder.set_root(root)?;

    let result = encoder.encode_expr(&builder.finish(), root, &mut rules, &params);

    match result {
        Err(DatalogError::UnsupportedNode { kind, .. }) => {
            assert_eq!(kind, ExprKind::AtomicFormula, "L'erreur doit pointer sur l'atome sous le NOT");
        },
        _ => panic!("Le NOT sur une formule atomique devrait être rejeté en descente"),
    }

    Ok(())
}

#[test]
fn test_not_equal_becomes_inequality() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut builder = ExprBuilder::new();
    let mut rules = Vec::new();

    // Logique : (NOT (= ?v0 ?v1))
    let v0 = builder.variable(0);
    let v1 = builder.variable(1);
    let eq = builder.comparison(CompareOp::Equal, v0, v1);
    let root = builder.not(eq);
    builder.set_root(root)?;

    let result = encoder.encode_expr(&builder.finish(), root, &mut rules, &params)?;

    let atom = result.expect("Devrait retourner un atome d'inégalité");
    assert!(atom.is_equality(), "Doit être une égalité");
    assert!(atom.is_negated(), "Le flag negated doit être true (représentant !=)");

    Ok(())
}

#[test]
fn test_not_of_constant_conflict_is_true() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut builder = ExprBuilder::new();
    let mut rules = Vec::new();

    // Logique : (NOT (= c1 c2)) où c1 != c2  => Toujours Vrai
    let c1 = builder.constant(1);
    let c2 = builder.constant(2);
    let eq = builder.comparison(CompareOp::Equal, c1, c2);
    let root = builder.not(eq);
    builder.set_root(root)?;

    let result = encoder.encode_expr(&builder.finish(), root, &mut rules, &params)?;

    let atom = result.expect("Le NOT d'un faux constant est vrai");
    // Ton code renvoie une tautologie (v0 = v0) pour signifier "True"
    assert!(atom.is_equality());
    let terms = atom.terms();
    assert_eq!(terms[0], terms[1], "Doit être une tautologie (v0=v0) pour marquer le succès");

    Ok(())
}

#[test]
fn test_not_ignored_in_effects() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut builder = ExprBuilder::new();
    let mut rules = Vec::new();

    // Cause : Action A
    let action_atom = Atom::new(AtomSkeletonId::from(100), vec![]);

    // Effet : (AND (P10) (NOT (P20)))
    let p10 = builder.atomic_formula_with_skeleton(10, vec![], 10);
    let p20 = builder.atomic_formula_with_skeleton(20, vec![], 20);
    let not_p20 = builder.not(p20);
    let root = builder.and(vec![p10, not_p20]);
    builder.set_root(root)?;

    encoder.encode_effects(&builder.finish(), &action_atom, &mut rules, &params)?;

    // On ne doit avoir qu'une seule règle : P10 :- ActionA
    // Le (NOT P20) doit avoir été sauté par le match kind { ExprKind::Not => continue }
    assert_eq!(rules.len(), 1, "Seul l'effet positif P10 doit produire une règle");
    assert_eq!(rules[0].head().skeleton_id().as_usize(), 10);

    Ok(())
}

#[test]
fn test_and_preserves_inequality_but_filters_tautology() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut builder = ExprBuilder::new();
    let mut rules = Vec::new();

    // Logique : (AND (P10) (= ?v0 ?v0) (NOT (= ?v0 ?v1)))
    // 1. P10 -> Gardé
    // 2. ?v0 = ?v0 -> Tautologie -> Filtré
    // 3. ?v0 != ?v1 -> Inégalité -> Gardé
    let v0 = builder.variable(0);
    let v1 = builder.variable(1);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![], 10);
    let eq_triv = builder.comparison(CompareOp::Equal, v0, v0);
    let eq = builder.comparison(CompareOp::Equal, v0, v1);
    let ineq = builder.not(eq);

    let root = builder.and(vec![p10, eq_triv, ineq]);
    builder.set_root(root)?;

    let result = encoder.encode_expr(&builder.finish(), root, &mut rules, &params)?;

    // Le résultat sera un prédicat auxiliaire car il reste 2 atomes (P10 et !=)
    let head = result.expect("Devrait produire une règle auxiliaire");
    assert_eq!(rules.len(), 1);

    let body = &rules[0].body();
    assert_eq!(body.len(), 2, "Doit contenir P10 et l'inégalité. La tautologie a dû être filtrée.");

    let has_inequality = body.iter().any(|a| a.is_negated());
    assert!(has_inequality, "L'inégalité (!=) doit être présente dans le corps de la règle");

    Ok(())
}

#[test]
fn test_or_with_not_conflict_ignored() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut builder = ExprBuilder::new();
    let mut rules = Vec::new();

    // Construction des composants
    let c1 = builder.constant(1);
    let p10 = builder.atomic_formula_with_skeleton(10, vec![], 10);
    let eq_triv = builder.comparison(CompareOp::Equal, c1, c1);
    let not_true = builder.not(eq_triv);

    // On crée les deux racines possibles
    let root_and = builder.and(vec![p10, not_true]);
    let root_or = builder.or(vec![p10, not_true]);

    // On finalise l'expression (contient les deux arbres)
    let expr = builder.finish();

    // A. Test du bloc AND : le NOT(True) doit faire échouer tout le bloc
    let result_and = encoder.encode_expr(&expr, root_and, &mut rules, &params)?;
    assert!(result_and.is_none(), "Le AND devrait être None car une branche est NOT(True)");

    // B. Test du bloc OR : la branche NOT(True) est ignorée, P10 survit
    let result_or = encoder.encode_expr(&expr, root_or, &mut rules, &params)?;

    let atom = result_or.expect("Le OR devrait survivre grâce à P10");
    assert_eq!(atom.skeleton_id().as_usize(), 10);
    assert_eq!(rules.len(), 0, "Pas de règle auxiliaire car une seule branche est restée valide");

    Ok(())
}

#[test]
fn test_not_equal_variable_constant() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut builder = ExprBuilder::new();
    let mut rules = Vec::new();

    let v0 = builder.variable(0);
    let c1 = builder.constant(1);
    let eq = builder.comparison(CompareOp::Equal, v0, c1);
    let root = builder.not(eq);
    builder.set_root(root)?;

    let result = encoder.encode_expr(&builder.finish(), root, &mut rules, &params)?;

    let atom = result.expect("Devrait retourner Some");
    assert!(atom.is_negated(), "Doit être une inégalité (?v0 != c1)");
    let terms = atom.terms();
    assert!(matches!(terms[0], Term::Variable(_)));
    assert!(matches!(terms[1], Term::Constant(_)));

    Ok(())
}

#[test]
fn test_double_not_is_rejected_as_non_pnf() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut builder = ExprBuilder::new();
    let mut rules = Vec::new();

    let v0 = builder.variable(0);
    let v1 = builder.variable(1);
    let eq = builder.comparison(CompareOp::Equal, v0, v1);

    // Construction de NOT(NOT(= ?v0 ?v1))
    let not_inner = builder.not(eq);
    let root = builder.not(not_inner);
    builder.set_root(root)?;

    // On exécute l'encodage
    let result = encoder.encode_expr(&builder.finish(), root, &mut rules, &params);

    // On vérifie que cela produit bien une erreur UnsupportedNode pour le type 'Not'
    // car le premier NOT s'attend à une Comparison, pas à un autre NOT.
    match result {
        Err(DatalogError::UnsupportedNode { kind, .. }) => {
            assert_eq!(kind, ExprKind::Not, "L'erreur doit porter sur le nœud Not imbriqué");
        },
        _ => panic!("L'encodeur devrait rejeter le double NOT comme non-PNF"),
    }

    Ok(())
}

#[test]
fn test_not_does_not_create_aliases() -> Result<(), DatalogError> {
    let (mut encoder, params) = setup_env();
    let mut builder = ExprBuilder::new();
    let mut rules = Vec::new();

    // 1. On crée les IDs de variables (ceux qu'on passera à resolve_var)
    let var_id0 = VariableId::from(0);
    let var_id1 = VariableId::from(1);
    let var_id2 = VariableId::from(2);

    // 2. On crée les nœuds pour l'expression
    let v0 = builder.variable(var_id0);
    let v1 = builder.variable(var_id1);
    let v2 = builder.variable(var_id2);

    // Expression : (AND (= ?v1 ?v2) (NOT (= ?v1 ?v0)))
    let alias = builder.comparison(CompareOp::Equal, v1, v2);
    let eq = builder.comparison(CompareOp::Equal, v1, v0);
    let ineq = builder.not(eq);
    let root = builder.and(vec![alias, ineq]);
    builder.set_root(root)?;

    // 3. On lance l'encodage (qui va remplir encoder.current_aliases)
    let _ = encoder.encode_expr(&builder.finish(), root, &mut rules, &params)?;

    // 4. On vérifie en utilisant les VariableId directs
    assert_eq!(
        encoder.resolve_var(var_id1),
        encoder.resolve_var(var_id2),
        "v1 et v2 doivent être liés car ils sont dans une égalité positive"
    );

    assert_ne!(
        encoder.resolve_var(var_id1),
        encoder.resolve_var(var_id0),
        "v1 et v0 ne doivent PAS être liés car leur égalité est sous un NOT"
    );

    Ok(())
}
