use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::passes::pnf::scratchpad::PnfScratchpad;
use crate::aiplan4rust::grounding::passes::pnf::{action, derived_predicate, expr, method};
use crate::aiplan4rust::lang::AtomSkeletonId;
use crate::aiplan4rust::lir::problem::NewLiftedProblem;

/// Fully applies the Positive Normal Form (PNF) transformation across the entire planning problem.
///
/// Convenience wrapper around [`to_pnf_with_scratchpad`] that allocates the temporary
/// scratchpad locally on the fly.
pub fn to_pnf(problem: &mut NewLiftedProblem) -> Result<Vec<AtomSkeletonId>, GroundingError> {
    let mut scratchpad = PnfScratchpad::new();
    to_pnf_with_scratchpad(problem, &mut scratchpad)
}

/// Fully applies the Positive Normal Form (PNF) transformation across the entire planning problem.
///
/// Extrait temporairement le store d'expressions du problème pour lever les contraintes d'emprunt (ownership)
/// et réutilise un unique `PfnScratchpad` pour garantir zéro allocation sur l'ensemble du processus.
pub fn to_pnf_with_scratchpad(
    problem: &mut NewLiftedProblem,
    scratchpad: &mut PnfScratchpad,
) -> Result<Vec<AtomSkeletonId>, GroundingError> {
    let mut negated_atoms = Vec::new();

    // --- 1. EXTRACTION DU STORE (Take Ownership) ---
    // Libère `problem` de ses emprunts liés au store pour nous permettre de muter
    // les actions, méthodes et prédicats dérivés de manière fluide.
    let mut store = problem.take_store();

    // --- 2. Global Constraints (Contraintes du Domaine & Problème) ---
    // Agissent comme des préconditions globales (is_effect = false)
    let old_domain_constraints = problem.domain_constraints();
    let new_domain_constraints = expr::to_pnf_with_scratchpad(
        old_domain_constraints,
        &mut store,
        &mut negated_atoms,
        scratchpad,
        false, // is_effect
    )?;
    problem.set_domain_constraints(new_domain_constraints);

    let old_problem_constraints = problem.problem_constraints();
    let new_problem_constraints = expr::to_pnf_with_scratchpad(
        old_problem_constraints,
        &mut store,
        &mut negated_atoms,
        scratchpad,
        false, // is_effect
    )?;
    problem.set_problem_constraints(new_problem_constraints);

    // --- 3. Lifted Definitions (Prédicats Dérivés) ---
    for derived in problem.derived_predicate_defs_mut() {
        derived_predicate::to_pnf_with_scratchpad(
            derived,
            &mut store,
            &mut negated_atoms,
            scratchpad,
        )?;
    }

    // --- 4. Lifted Definitions (Actions) ---
    for action_def in problem.action_defs_mut() {
        action::to_pnf_with_scratchpad(action_def, &mut store, &mut negated_atoms, scratchpad)?;
    }

    // --- 5. Lifted Definitions (Méthodes HTN) ---
    for method_def in problem.method_defs_mut() {
        method::to_pnf_with_scratchpad(method_def, &mut store, &mut negated_atoms, scratchpad)?;
    }

    // --- 6. Problem Instance Specifics (But & Métriques) ---
    let old_goal = problem.goal();
    let new_goal = expr::to_pnf_with_scratchpad(
        old_goal,
        &mut store,
        &mut negated_atoms,
        scratchpad,
        false, // Le but est une condition
    )?;
    problem.set_goal(new_goal);

    let old_metric = problem.metric_spec();
    let new_metric = expr::to_pnf_with_scratchpad(
        old_metric,
        &mut store,
        &mut negated_atoms,
        scratchpad,
        false, // Les métriques ne sont pas des effets
    )?;
    problem.set_metric_spec(new_metric);

    // --- 7. HTN Initial Task Network (Contraintes du réseau initial) ---
    let current_htn_constraints = problem
        .initial_task_network()
        .task_network()
        .logical_constraints();
    let new_htn_constraints = expr::to_pnf_with_scratchpad(
        current_htn_constraints,
        &mut store,
        &mut negated_atoms,
        scratchpad,
        false, // Contraintes initiales = conditions
    )?;
    problem
        .initial_task_network_mut()
        .task_network_mut()
        .set_logical_constraints(new_htn_constraints);

    // --- 8. RÉINJECTION DU STORE (Restore Ownership) ---
    // Le store, enrichi et réécrit sans négations structurelles, est restitué au problème.
    problem.set_store(store);

    Ok(negated_atoms)
}
