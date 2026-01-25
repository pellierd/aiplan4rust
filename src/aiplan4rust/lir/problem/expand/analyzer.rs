use std::collections::HashSet;
use crate::aiplan4rust::lang::BinaryComp;
use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprKind};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::expand::inertia::Inertia;
use crate::aiplan4rust::lir::problem::expand::inertia_table::InertiaTable;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::syntax::tree::SyntaxNode;

/// Analyse le problème pour identifier l'inertie des prédicats et des fonctions.
pub fn analyze_inertia(problem: &LiftedProblem) -> Result<InertiaTable, LirError> {
    let mut fluent_predicates = HashSet::new();
    let mut fluent_functions = HashSet::new();
    let mut initial_predicates = HashSet::new();
    let mut initial_functions = HashSet::new();

    // 1. Scan des effets (Actions et Actions Duratives)
    // On identifie tout ce qui change via une action
    for action in problem.actions() {
        collect_fluents_from_expr(action.effect(), problem, &mut fluent_predicates, &mut fluent_functions);
    }

    for d_action in problem.durative_actions() {
        collect_fluents_from_expr(d_action.effect(), problem, &mut fluent_predicates, &mut fluent_functions);
    }

    // 2. Scan de l'état initial
    // On identifie les faits statiques (T=0) et les fluents temporels (at <t> ...)
    collect_initial_facts(
        problem.init(),
        problem,
        &mut initial_predicates,
        &mut initial_functions,
        &mut fluent_predicates, // On complète les fluents avec les Timed Initial Literals
        &mut fluent_functions,
    )?;

    let mut table = InertiaTable::new();

    // 3. Catégorisation des Prédicats
    for (idx, _) in problem.predicates().iter().enumerate() {
        let inertia = if fluent_predicates.contains(&idx) {
            Inertia::Fluent
        } else if initial_predicates.contains(&idx) {
            Inertia::Positive // Jamais modifié et présent à T=0
        } else {
            Inertia::Negative // Jamais modifié et absent à T=0
        };
        table.insert_predicate(idx, inertia);
    }

    // 4. Catégorisation des Fonctions
    for (idx, _) in problem.functions().iter().enumerate() {
        let inertia = if fluent_functions.contains(&idx) {
            Inertia::Fluent
        } else if initial_functions.contains(&idx) {
            Inertia::Positive // Valeur constante définie dans l'init
        } else {
            // Une fonction jamais initialisée est techniquement indéfinie (ou 0)
            // et ne changera jamais : on la marque Negative.
            Inertia::Negative
        };
        table.insert_function(idx, inertia);
    }

    Ok(table)
}

/// Parcourt un effet pour identifier les prédicats et fonctions modifiés (Fluents).
pub fn collect_fluents_from_expr(
    expr: &Expr,
    _problem: &LiftedProblem, // Ce paramètre n'est plus strictement nécessaire pour le binding !
    fluent_predicates: &mut HashSet<usize>,
    fluent_functions: &mut HashSet<usize>,
) {
    let Some(root_id) = expr.root_id() else { return };
    let mut stack = vec![root_id];

    while let Some(node_id) = stack.pop() {
        let node = match expr.try_node(node_id) {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Accès direct au contenu résolu
        match node.content() {
            ExprContent::ResolvedIdent(_ident, declaration) => {
                // On regarde le Kind du nœud pour savoir où classer l'index
                match node.kind() {
                    ExprKind::Predicate => {
                        fluent_predicates.insert(*declaration);
                    }
                    ExprKind::FunctionSymbol => {
                        fluent_functions.insert(*declaration);
                    }
                    _ => {}
                }
            }
            _ => {
                // Si c'est un "When", on ne veut traiter que l'effet (index 1)
                if node.kind() == ExprKind::When {
                    if let Some(&effect_id) = node.children().get(1) {
                        stack.push(effect_id);
                    }
                } else {
                    // Pour tout le reste (And, Not, Quantifiers), on descend
                    for &child_id in node.children() {
                        stack.push(child_id);
                    }
                }
            }
        }
    }
}

fn collect_initial_facts(
    init_expr: &Expr,
    _problem: &LiftedProblem,
    initial_predicates: &mut HashSet<usize>,
    initial_functions: &mut HashSet<usize>,
    fluent_predicates: &mut HashSet<usize>,
    fluent_functions: &mut HashSet<usize>,
) -> Result<(), LirError> {
    let Some(root_id) = init_expr.root_id() else { return Ok(()); };
    let mut stack = vec![(root_id, false)];

    while let Some((node_id, is_timed)) = stack.pop() {
        let node = init_expr.try_node(node_id)?;
        let current_is_timed = is_timed || node.kind() == ExprKind::TimedInitialLiteral;

        match node.content() {
            ExprContent::ResolvedIdent(_ident, declaration) => {
                let idx = *declaration;
                match node.kind() {
                    ExprKind::Predicate => {
                        if current_is_timed { fluent_predicates.insert(idx); }
                        else { initial_predicates.insert(idx); }
                    }
                    ExprKind::FunctionSymbol => {
                        if current_is_timed { fluent_functions.insert(idx); }
                        else { initial_functions.insert(idx); }
                    }
                    _ => {}
                }
            }
            _ => {
                // On continue le parcours pour trouver les identifiants
                for &child_id in node.children().iter().rev() {
                    stack.push((child_id, current_is_timed));
                }
            }
        }
    }
    Ok(())
}
