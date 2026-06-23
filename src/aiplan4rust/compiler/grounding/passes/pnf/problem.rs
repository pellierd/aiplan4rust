use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::pnf::scratchpad::PnfScratchpad;
use crate::aiplan4rust::compiler::grounding::passes::pnf::{
    action, derived_predicate, expr, method,
};
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::support::lang::AtomSkeletonId;

/// Fully applies the Prenex Normal Form (PNF) transformation across the entire planning problem.
///
/// Convenient entry point wrapper around [`to_pnf_with_scratchpad`] that automatically
/// allocates a temporary, local [`PnfScratchpad`] on the fly.
///
/// # Layout and Optimizations
///
/// While this function is ideal for one-off conversions or isolated test cases,
/// batch-processing pipelines handling massive sets of problems should favor
/// calling [`to_pnf_with_scratchpad`] directly with a single, retained scratchpad
/// to maximize performance and guarantee zero heap allocations.
pub fn to_pnf(problem: &mut LiftedProblem) -> Result<Vec<AtomSkeletonId>, GroundingError> {
    // Allocate a localized buffer stack and memoization cache for this single pass
    let mut scratchpad = PnfScratchpad::new();
    to_pnf_with_scratchpad(problem, &mut scratchpad)
}

/// Fully applies the Prenex Normal Form (PNF) transformation across the entire planning problem.
///
/// This master function processes every expression sub-tree within the problem instance
/// (domain constraints, actions, HTN methods, derived predicates, goals, metrics, and task networks),
/// rewriting them into their canonical PNF representation.
///
/// # Layout and Optimizations
///
/// * **Ownership Demultiplexing**: Temporarily takes ownership of the underlying [`ExprStore`] via
///   `problem.take_store()`. This bypasses Rust's aliasing rules (Borrow Checker constraints),
///   allowing fluid, simultaneous mutation of both the definition structures and the shared store.
/// * **Zero-Allocation Pipeline**: Reuses a single, pre-allocated [`PnfScratchpad`] throughout
///   the entire sequence to guarantee completely zero dynamic reallocations.
/// * **Global Canonical Tracking**: Accumulates all absorbed negated atoms into a single, comprehensive
///   vector returned upon successful completion.
pub fn to_pnf_with_scratchpad(
    problem: &mut LiftedProblem,
    scratchpad: &mut PnfScratchpad,
) -> Result<Vec<AtomSkeletonId>, GroundingError> {
    let mut negated_atoms = Vec::new();

    // --- 1. STORE EXTRACTION (Take Ownership) ---
    // Detaches the expression store from the problem container to allow mutable access
    // across definitions without violating single-ownership constraints.
    let mut store = problem.take_store();

    // --- 2. GLOBAL CONSTRAINTS ---
    // Domain and problem constraints act as global preconditions (`is_effect = false`)
    let old_domain_constraints = problem.domain_constraints();
    let new_domain_constraints = expr::to_pnf_with_scratchpad(
        old_domain_constraints,
        &mut store,
        &mut negated_atoms,
        scratchpad,
        false, // is_effect = false
    )?;
    problem.set_domain_constraints(new_domain_constraints);

    let old_problem_constraints = problem.problem_constraints();
    let new_problem_constraints = expr::to_pnf_with_scratchpad(
        old_problem_constraints,
        &mut store,
        &mut negated_atoms,
        scratchpad,
        false, // is_effect = false
    )?;
    problem.set_problem_constraints(new_problem_constraints);

    // --- 3. LIFTED DEFINITIONS (Derived Predicates) ---
    for derived in problem.derived_predicate_defs_mut() {
        derived_predicate::to_pnf_with_scratchpad(
            derived,
            &mut store,
            &mut negated_atoms,
            scratchpad,
        )?;
    }

    // --- 4. LIFTED DEFINITIONS (Actions) ---
    for action_def in problem.action_defs_mut() {
        action::to_pnf_with_scratchpad(action_def, &mut store, &mut negated_atoms, scratchpad)?;
    }

    // --- 5. LIFTED DEFINITIONS (HTN Methods) ---
    for method_def in problem.method_defs_mut() {
        method::to_pnf_with_scratchpad(method_def, &mut store, &mut negated_atoms, scratchpad)?;
    }

    // --- 6. PROBLEM INSTANCE SPECIFICS (Goal & Metrics) ---
    let old_goal = problem.goal();
    let new_goal = expr::to_pnf_with_scratchpad(
        old_goal,
        &mut store,
        &mut negated_atoms,
        scratchpad,
        false, // Goals act as conditions
    )?;
    problem.set_goal(new_goal);

    let old_metric = problem.metric_spec();
    let new_metric = expr::to_pnf_with_scratchpad(
        old_metric,
        &mut store,
        &mut negated_atoms,
        scratchpad,
        false, // Metric calculations do not evaluate to side effects
    )?;
    problem.set_metric_spec(new_metric);

    // --- 7. HTN INITIAL TASK NETWORK ---
    let current_htn_constraints = problem
        .initial_task_network()
        .task_network()
        .logical_constraints();
    let new_htn_constraints = expr::to_pnf_with_scratchpad(
        current_htn_constraints,
        &mut store,
        &mut negated_atoms,
        scratchpad,
        false, // Initial constraints evaluate as conditions
    )?;
    problem
        .initial_task_network_mut()
        .task_network_mut()
        .set_logical_constraints(new_htn_constraints);

    // --- 8. STORE RE-INJECTION (Restore Ownership) ---
    // The store, now populated with rewritten and optimized hash-consed expression paths,
    // is safely returned to the problem structure.
    problem.set_store(store);

    Ok(negated_atoms)
}
