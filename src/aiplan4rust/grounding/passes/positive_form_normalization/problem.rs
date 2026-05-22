use super::{action, derived_predicate, expr, method};
use crate::aiplan4rust::lang::AtomSkeletonId;
use crate::aiplan4rust::lir::old::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::old::problem::LiftedProblem;

pub fn to_pnf(problem: &mut LiftedProblem) -> Result<Vec<AtomSkeletonId>, ExprOpError> {
    let max_preds = problem.predicate_defs().len();
    let nb_components = problem.action_defs().len()
        + problem.method_defs().len()
        + problem.derived_predicate_defs().len()
        + 2; // +2 pour le But et les Contraintes Globales

    // 1. Initialisation des buffers de travail.
    // Capacité maximale théorique pour éviter tout realloc pendant la collecte.
    let mut negated_atoms = Vec::with_capacity(nb_components * max_preds);

    // Le scratchpad peut contenir jusqu'à 2x max_preds (ex: precond + effets)
    // avant son dédoublonnement local.
    let mut scratchpad = Vec::with_capacity(max_preds * 2);

    // La pile DFS pour la traversée d'expression (32-64 est généralement suffisant,
    // mais elle grandira dynamiquement si l'arbre est très profond).
    let mut dfs_stack = Vec::with_capacity(64);

    // --- 1. Global Constraints ---
    if let Some(root) = problem.domain_constraints_mut().root_id() {
        scratchpad.clear();
        expr::to_pnf(
            root,
            problem.domain_constraints_mut(),
            &mut scratchpad,
            &mut dfs_stack,
            false,
        )?;
        negated_atoms.extend_from_slice(&scratchpad);
    }
    if let Some(root) = problem.problem_constraints_mut().root_id() {
        scratchpad.clear();
        expr::to_pnf(
            root,
            problem.problem_constraints_mut(),
            &mut scratchpad,
            &mut dfs_stack,
            false,
        )?;
        negated_atoms.extend_from_slice(&scratchpad);
    }

    // --- 2. Lifted Definitions (Actions, Methods, Derived) ---
    for derived in problem.derived_predicate_defs_mut() {
        derived_predicate::to_pnf(derived, &mut negated_atoms, &mut scratchpad, &mut dfs_stack)?;
    }

    for action_def in problem.action_defs_mut() {
        action::to_pnf(
            action_def,
            &mut negated_atoms,
            &mut scratchpad,
            &mut dfs_stack,
        )?;
    }

    for method_def in problem.method_defs_mut() {
        method::to_pnf(
            method_def,
            &mut negated_atoms,
            &mut scratchpad,
            &mut dfs_stack,
        )?;
    }

    // --- 3. Problem Instance Specifics (Goal) ---
    if let Some(root) = problem.goal_mut().root_id() {
        scratchpad.clear();
        expr::to_pnf(
            root,
            problem.goal_mut(),
            &mut scratchpad,
            &mut dfs_stack,
            false,
        )?;
        negated_atoms.extend_from_slice(&scratchpad);
    }

    // --- 4. Final Deduplication ---
    // Indispensable car les différents composants peuvent partager les mêmes prédicats niés.
    negated_atoms.sort_unstable();
    negated_atoms.dedup();

    // On réduit la mémoire au strict nécessaire avant de retourner le vecteur au moteur Datalog.
    negated_atoms.shrink_to_fit();

    Ok(negated_atoms)
}
