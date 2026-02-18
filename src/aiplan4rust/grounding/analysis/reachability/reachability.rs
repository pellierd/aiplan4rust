/*use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::registry::fluent::FluentRegistry;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::grounding::iterator::DomainIterator;
use crate::aiplan4rust::grounding::registry::value::ValueRegistry;
use crate::aiplan4rust::lang::ConstantId;
use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::tree::NodeId;

/// Calcule l'ensemble des valeurs (constantes et fluents) atteignables pour chaque type.
/// C'est le cœur du grounding dynamique qui évite l'explosion combinatoire initiale.
pub fn compute_reachability(
    problem: &LiftedProblem,
    fluent_reg: &mut FluentRegistry,
) -> Result<ValueRegistry, GroundingError> {
    // 1. Initialisation : Objets statiques + Fluents de l'état initial
    let mut store = ValueRegistry::from_problem(problem, fluent_reg)?;

    let mut global_changed = true;
    let mut round = 0;

    // 2. Boucle de point fixe (Relaxed Planning Graph)
    while global_changed {
        global_changed = false;
        round += 1;

        let mut round_has_discoveries = false;

        for action in problem.action_defs() {
            // Récupération des domaines actuels pour les paramètres de l'action
            let param_domains: Vec<_> = action
                .parameters()
                .iter()
                .map(|p| store.get_domain_of_type(p.ty()))
                .collect();

            // Création de l'itérateur sur le produit cartésien des domaines
            let mut it = DomainIterator::new(param_domains)?;

            while let Some(combo) = it.next() {
                // ÉVALUATION DES PRÉCONDITIONS (Approche optimiste pour l'attaignabilité)
                if action.is_potentially_applicable(problem, &combo, &store)? {

                    // ANALYSE DES EFFETS via l'arbre syntaxique
                    let effect_expr = action.effect();

                    // On parcourt l'arbre des effets (preorder) pour découvrir de nouveaux objets
                    for node in effect_expr.preorder().values() {

                        match node.kind() {
                            // Cas de l'Assignation : le cœur de la découverte d'objets
                            ExprKind::Assign => {
                                let children = node.children();

                                if children.len() >= 2 {
                                    let lhs_id = children[0]; // Le nœud du Fluent (FunctionTerm)
                                    let rhs_id = children[1]; // Le nœud de la Valeur/Expression

                                    let lhs_node = effect_expr.try_node(lhs_id)?;

                                    // 1. Récupérer la définition de la fonction pour le type de retour
                                    let skeleton_id = lhs_node.try_function_skeleton()?;
                                    let function_def = problem.try_get_function(skeleton_id)?;

                                    // Les types primitifs dans lesquels cet objet doit être enregistré
                                    let target_types = function_def.ty().members();

                                    // 2. Tenter d'extraire un objet du côté droit (RHS)
                                    if let Some(obj_id) = extract_object_id_from_node(
                                        rhs_id,
                                        effect_expr,
                                        &combo,
                                        problem,
                                        fluent_reg
                                    )? {
                                        // On utilise la méthode register du store
                                        if store.register(obj_id, target_types) {
                                            global_changed = true;
                                            round_has_discoveries = true;
                                        }
                                    }
                                }
                            }
                            // Les Forall et When sont parcourus automatiquement par preorder()
                            _ => {}
                        }
                    }
                }
            }
        }

        // 3. Synchronisation : Mise à jour des domaines pour le prochain round
        if round_has_discoveries {
            store.sync();
        }

        // Sécurité contre les boucles infinies
        /*if round > 500 {
            return Err(GroundingError::ReachabilityDivergence(
                "L'étude d'attaignabilité n'a pas convergé.".to_string()
            ));
        }*/
    }

    Ok(store)
}

/// Helper pour résoudre un nœud en ObjectId (Constant ou Fluent) selon le contexte.
fn extract_object_id_from_node(
    node_id: NodeId,
    expr: &Expr,
    combo: &[ConstantId],
    problem: &LiftedProblem,
    fluent_reg: &mut FluentRegistry
) -> Result<Option<ConstantId>, GroundingError> {
    let node = expr.try_node(node_id)?;

    match node.kind() {
        // C'est une constante (ex: 'city1')
        ExprKind::Constant => {
            let cid = node.try_constant()?;
            Ok(Some(cid))
        },
        // C'est un paramètre d'action (ex: '?p') remplacé par la valeur dans 'combo'
        ExprKind::Variable => {
            let var_idx = node.try_variable()?;
            let cid = combo[var_idx];
            Ok(Some(cid))
        },
        // C'est un fluent (ex: '(at ?p)')
        ExprKind::FunctionTerm => {
            // Utilisation de la méthode statique définie dans ValueRegistry
            let fid = ValueRegistry::extract_fluent_from_node(node_id, expr, problem, fluent_reg)?;
            Ok(Some(ArgumentId::Fluent(fid)))
        },
        _ => Ok(None)
    }
}*/
