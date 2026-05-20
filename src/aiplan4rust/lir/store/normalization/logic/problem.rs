use crate::aiplan4rust::lir::store::expr::ops::rewriting::Scratchpad;
use crate::aiplan4rust::lir::store::expr::ExprStore;
use crate::aiplan4rust::lir::store::normalization::error::NormalizationError;
use crate::aiplan4rust::lir::store::normalization::logic::{action, derived_predicate, method};
use crate::aiplan4rust::lir::store::normalization::logic::{expr, task_network};
use crate::aiplan4rust::lir::store::problem::NewLiftedProblem;

/// Normalizes all logical expression components of a `LiftedProblem`.
///
/// This function coordinates the canonical expression normalization pipeline across:
/// - Global problem-level expressions (`goal`, `domain_constraints`, `problem_constraints`, `metric_spec`)
/// - All declared `DerivedPredicateDef` instances
/// - All declared `ActionDef` instances (both instant and temporal/durative)
/// - All HTN `MethodDef` decomposition structures
/// - The `InitialTaskNetwork` constraints
///
/// # Arguments
///
/// * `problem` - A mutable reference to the `LiftedProblem` to normalize.
/// * `store` - The central `ExprStore` containing the expression nodes.
/// * `scratch` - A reusable `Scratchpad` to avoid heap allocations during rewrite passes.
///
/// # Errors
///
/// Returns an `ExprOpErrorHC` if normalization or simplification fails on any
/// expression sub-tree within the problem.
pub fn normalize(
    problem: &mut NewLiftedProblem,
    store: &mut ExprStore,
    scratch: &mut Scratchpad,
) -> Result<(), NormalizationError> {
    // 1. Global Problem-level expressions (non-durative semantics context)
    println!("Normalizing Goal expressions...\n {}", problem.goal());

    let normalized_goal = expr::normalize(problem.goal(), store, scratch, false)?;
    problem.set_goal(normalized_goal);

    println!("Normalizing Domain Constraints expressions...");
    let normalized_dom_constraints =
        expr::normalize(problem.domain_constraints(), store, scratch, false)?;
    problem.set_domain_constraints(normalized_dom_constraints);

    println!("Normalizing Problem Constraints expressions...");
    let normalized_prob_constraints =
        expr::normalize(problem.problem_constraints(), store, scratch, false)?;
    problem.set_problem_constraints(normalized_prob_constraints);

    println!("Normalizing Metric expressions...");

    let normalized_metric = expr::normalize(problem.metric_spec(), store, scratch, false)?;
    problem.set_metric_spec(normalized_metric);

    // 2. Normalize all derived predicates
    for derived_predicate in problem.derived_predicate_defs_mut() {
        println!("Normalizing Derive predicate expressions...");
        derived_predicate::normalize(derived_predicate, store, scratch)?;
    }

    // 3. Normalize all actions (internally manages durative vs non-durative branches)
    for action in problem.action_defs_mut() {
        println!("Normalizing Action...");
        action::normalize(action, store, scratch)?;
    }

    // 4. Normalize all HTN methods
    for method in problem.method_defs_mut() {
        method::normalize(method, store, scratch)?;
    }

    // --- 5. Initial Task Network Constraints ---
    // Normalize the initial task network's logical constraints.
    task_network::normalize(
        problem.initial_task_network_mut().task_network_mut(),
        store,
        scratch,
    )?;

    Ok(())
}
