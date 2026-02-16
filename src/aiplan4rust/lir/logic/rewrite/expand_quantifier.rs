/*use std::collections::HashMap;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{ObjectID, TypeID, VariableID};
use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::lir::logic::LogicError;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::tree::NodeId;

pub fn expand_quantifiers(
    expr: &mut Expr,
    problem: &ProblemData,
) -> Result<(), LogicError> {
    if expr.is_empty() {
        return Ok(());
    }

    let root_id = expr.try_root_node()?;

    // Pile pour le parcours : (ID du nœud, est_ce_qu_on_a_traité_les_enfants)
    let mut stack = vec![(root_id, false)];

    while let Some((current_node, children_processed)) = stack.pop() {
        if !children_processed {
            // Premier passage : on remet le nœud sur la pile avec le marqueur à 'true'
            // puis on ajoute ses enfants par-dessus.
            stack.push((current_node, true));

            let children = current_node?.children().to_vec();
            // On les ajoute à l'envers pour garder l'ordre original si nécessaire
            for &child_id in children.iter().rev() {
                stack.push((child_id, false));
            }
        } else {
            // Deuxième passage : les enfants ont été traités, on peut "expander" ce nœud
            let kind = expr.try_node(current_node)?.kind();

            if kind == ExprKind::Forall || kind == ExprKind::Exists {
                let vars = current_node.try_quantifier_vars()?;
                let body_id = current_node.try_child(0)?;
                let combinations = generate_combinations(&vars, problem)?;

                let mut instances = Vec::new();
                for combo in combinations {
                    let env: HashMap<VariableID, ObjectID> = combo.into_iter().collect();

                    // On clone et on substitue
                    let instance_id = expr.clone_subtree(body_id)?;
                    expr.substitute(instance_id, &env)?;

                    instances.push(instance_id);
                }

                // Transformation finale du nœud d'origine
                if kind == ExprKind::Forall {
                    expr.set_to_and(current_node, instances)?;
                } else {
                    expr.set_to_or(current_node, instances)?;
                }
            }
        }
    }

    Ok(())
}


/// Génère toutes les combinaisons possibles d'objets pour une liste de variables typées.
/// Retourne un vecteur de "maps" (sous forme de Vec de paires pour l'efficacité).
fn generate_combinations(
    vars: &[(VariableID, TypeID)],
    problem: &LiftedProblem,
) -> Result<Vec<Vec<(VariableID, ObjectID)>>, LogicError> {
    // On commence avec une seule combinaison vide
    let mut results: Vec<Vec<(VariableID, ObjectID)>> = vec![vec![]];

    for (var_id, type_id) in vars {
        // 1. Récupérer tous les objets qui correspondent au type de la variable
        let objects = problem.(*type_id);

        // Si un type n'a pas d'objets, le produit cartésien est vide
        if objects.is_empty() {
            return Ok(Vec::new());
        }

        let mut next_step_results = Vec::with_capacity(results.len() * objects.len());

        // 2. Pour chaque combinaison déjà construite...
        for existing_combo in &results {
            // 3. ... et pour chaque objet possible pour la variable actuelle
            for &obj_id in objects {
                let mut new_combo = existing_combo.clone();
                new_combo.push((*var_id, obj_id));
                next_step_results.push(new_combo);
            }
        }
        results = next_step_results;
    }

    Ok(results)
}*/
