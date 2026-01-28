use std::collections::HashSet;
use crate::aiplan4rust::lang::{BinaryComp, FunctionID, PredicateID};
use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprKind, Resolution};
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
    // On identifie tout ce qui est modifié par une action.
    for action in problem.actions() {
        collect_fluents_from_expr(
            action.effect(),
            problem, // Optionnel selon ta signature finale
            &mut fluent_predicates,
            &mut fluent_functions
        )?;
    }

    for d_action in problem.durative_actions() {
        collect_fluents_from_expr(
            d_action.effect(),
            problem,
            &mut fluent_predicates,
            &mut fluent_functions
        )?;
    }

    // 2. Scan de l'état initial
    // On identifie les faits statiques (Initiaux) et les TILs (Fluents temporels)
    collect_initial_facts(
        problem.init(),
        problem,
        &mut initial_predicates,
        &mut initial_functions,
        &mut fluent_predicates,
        &mut fluent_functions,
    )?;

    let mut table = InertiaTable::new();

    // 3. Catégorisation des Prédicats
    // On utilise les IDs typés pour correspondre aux HashSets
    for (idx, _) in problem.predicates().iter().enumerate() {
        let pred_id = PredicateID::new(idx); // Conversion vers ton type ID

        let inertia = if fluent_predicates.contains(&pred_id) {
            Inertia::Fluent
        } else if initial_predicates.contains(&pred_id) {
            Inertia::Positive // Statique : présent à T=0 et jamais modifié
        } else {
            Inertia::Negative // Statique : absent à T=0 et jamais modifié
        };
        table.insert_predicate(pred_id, inertia);
    }

    // 4. Catégorisation des Fonctions
    for (idx, _) in problem.functions().iter().enumerate() {
        let func_id = FunctionID::new(idx);

        let inertia = if fluent_functions.contains(&func_id) {
            Inertia::Fluent
        } else if initial_functions.contains(&func_id) {
            Inertia::Positive // Constante : définie à l'init et jamais modifiée
        } else {
            Inertia::Negative // Indéfinie : jamais initialisée ni modifiée
        };
        table.insert_function(func_id, inertia);
    }

    Ok(table)
}

/// Parcourt un effet pour identifier les prédicats et fonctions modifiés (Fluents).
pub fn collect_fluents_from_expr(
    expr: &Expr,
    _problem: &LiftedProblem,
    fluent_predicates: &mut HashSet<PredicateID>, // Utilise tes types ID pour plus de clarté
    fluent_functions: &mut HashSet<FunctionID>,
) -> Result<(), LirError> {
    let root_id = expr.try_root_id()?;
    let mut stack = vec![root_id];

    while let Some(node_id) = stack.pop() {
        let node = expr.try_node(node_id)?;

        // 1. On regarde la résolution du nœud
        match node.resolution() {
            Resolution::Predicate(p_id) => {
                fluent_predicates.insert(*p_id);
            }
            Resolution::Function(f_id, _type_id) => {
                fluent_functions.insert(*f_id);
            }
            _ => {
                // Pas une feuille résolue (ou Paramètre/Constante), on continue le parcours

                // Gestion spécifique du "When" (Conditionnelle)
                // Dans un effet, on ne collecte les fluents que dans l'effet, pas la condition
                if node.kind() == ExprKind::When {
                    if let Some(&effect_id) = node.children().get(1) {
                        stack.push(effect_id);
                    }
                } else {
                    // Pour tout le reste (And, Not, Quantifiers, etc.), on descend dans les enfants
                    for &child_id in node.children() {
                        stack.push(child_id);
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn collect_initial_facts(
    init_expr: &Expr,
    _problem: &LiftedProblem,
    initial_predicates: &mut HashSet<PredicateID>,
    initial_functions: &mut HashSet<FunctionID>,
    fluent_predicates: &mut HashSet<PredicateID>,
    fluent_functions: &mut HashSet<FunctionID>,
) -> Result<(), LirError> {
    let Some(root_id) = init_expr.root_id() else { return Ok(()); };
    let mut stack = vec![(root_id, false)];

    while let Some((node_id, is_timed)) = stack.pop() {
        let node = init_expr.try_node(node_id)?;

        // On détecte si on entre dans un "Timed Initial Literal" (TIL)
        let current_is_timed = is_timed || node.kind() == ExprKind::TimedInitialLiteral;

        // On extrait la résolution sémantique
        match node.resolution() {
            Resolution::Predicate(p_id) => {
                if current_is_timed {
                    fluent_predicates.insert(*p_id);
                } else {
                    initial_predicates.insert(*p_id);
                }
            }
            Resolution::Function(f_id, _type_id) => {
                if current_is_timed {
                    fluent_functions.insert(*f_id);
                } else {
                    initial_functions.insert(*f_id);
                }
            }
            _ => {
                // Ce n'est pas un symbole résolu (c'est peut-être un AND, NOT, =, AT START, etc.)
                // On continue le parcours dans les enfants pour trouver les symboles.
                // .rev() est utilisé pour maintenir l'ordre de parcours original si nécessaire.
                for &child_id in node.children().iter().rev() {
                    stack.push((child_id, current_is_timed));
                }
            }
        }
    }
    Ok(())
}
