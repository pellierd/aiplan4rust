/*use crate::aiplan4rust::arena::ArenaNode;
use std::collections::HashMap;
use crate::aiplan4rust::grounding::analysis::inertia::analyzer;
use crate::aiplan4rust::grounding::analysis::inertia::registry::InertiaRegistry;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::iterator::DomainIterator;
use crate::aiplan4rust::grounding::problem::value_domain::ValueDomain;
use crate::aiplan4rust::lang::{ObjectID, VariableID};
use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprError, ExprKind};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::logic::engine::LogicEngine;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::tree::NodeId;

/// Fonction récursive qui transforme l'expression en place.
pub fn expand(
    expr: &mut Expr,
    domains: &[ValueDomain],
    registry: &InertiaRegistry,
) -> Result<(), GroundingError> {
    // 1. On fige l'ordre des IDs existants au début.
    // L'expansion va ajouter de nouveaux nœuds à la fin de l'arène,
    // mais ils ne seront pas visités par cette boucle (ce qui est voulu).
    let ids: Vec<NodeId> = expr.postorder().ids().map(|(id, _node)| id).collect();

    for node_id in ids {
        // On récupère le kind pour savoir si on doit agir
        let kind = expr.try_node_kind(node_id)?;

        match kind {
            ExprKind::Forall | ExprKind::Exists => {
                expand_quantifier(expr, node_id, domains, registry)?;
            }
            _ => {
                // Pour les autres nœuds, le post-order garantit
                // que leurs enfants ont déjà été traités.
            }
        }
    }

    Ok(())
}

pub fn expand_quantifier(
    expr: &mut Expr,
    node_id: NodeId,
    domains: &[ValueDomain],
    registry: &InertiaRegistry,
) -> Result<(), GroundingError> {
    // --- 1. EXTRACTION DES DONNÉES DU NŒUD ---
    let (variables, body_id, is_forall) = {
        let node = expr.try_node(node_id)?;
        let vars = node.content().try_quantifier_vars()?.clone();
        let body = node.try_child(0)?;
        let forall = node.kind() == ExprKind::Forall;
        (vars, body, forall)
    };

    // Construction des domaines pour chaque variable (Union des types si nécessaire)
    let var_domains: Vec<ValueDomain> = variables
        .iter()
        .map(|v| {
            v.ty().members().iter().fold(ValueDomain::new(), |acc, &type_id| {
                acc.union(&domains[type_id.as_usize()])
            })
        })
        .collect();

    // --- 2. INITIALISATION DE L'ITÉRATEUR ---
    // On passe les références des domaines à l'itérateur
    let mut iterator = DomainIterator::new(var_domains.iter().collect())?;

    // Gestion du cas domaine vide (géré nativement par l'itérateur via exhausted)
    if !iterator.has_next() && variables.len() > 0 {
        expr.set_to_bool(node_id, is_forall)?;
        return Ok(());
    }

    // --- 3. GÉNÉRATION ET FILTRAGE VIA L'ITÉRATEUR ---
    let mut instances = Vec::new();
    let mut env = HashMap::with_capacity(variables.len());

    // On utilise while let pour boucler sur l'itérateur performant
    while let Some(combo) = iterator.next() {
        // Préparation de l'environnement de substitution
        env.clear();
        for (i, &arg_id) in combo.iter().enumerate() {
            env.insert(variables[i].symbol(), arg_id);
        }

        // ÉVALUATION PARTIELLE VIA L'INERTIE
        // C'est ici que l'itérateur brille : on peut "skip" des branches entières
        match registry.evaluate_predicate_with_env(body_id, expr, &env)? {
            Some(true) => {
                if !is_forall {
                    // EXISTS + TRUE = Le bloc entier est TRUE
                    expr.set_to_bool(node_id, true)?;
                    return Ok(());
                }
                // FORALL + TRUE : Cette branche est validée, on passe à la suite
            }
            Some(false) => {
                if is_forall {
                    // FORALL + FALSE = Le bloc entier est FALSE
                    expr.set_to_bool(node_id, false)?;
                    return Ok(());
                }
                // EXISTS + FALSE : Cette branche ne sert à rien, on passe à la suite
            }
            None => {
                // L'inertie ne peut pas trancher -> On crée l'instance réelle
                let instance_id = expr.clone_subtree(body_id)?;
                expr.substitute(instance_id, &env)?;
                instances.push(instance_id);
            }
        }
    }

    // --- 4. MUTATION FINALE DU NŒUD ---
    if instances.is_empty() {
        // Si aucune instance n'est restée (ex: Forall dont toutes les branches étaient True)
        expr.set_to_bool(node_id, is_forall)?;
    } else {
        let new_kind = if is_forall { ExprKind::And } else { ExprKind::Or };
        let node_mut = expr.try_node_mut(node_id)?;

        node_mut.set_kind(new_kind);
        node_mut.set_content(ExprContent::None);
        *node_mut.children_mut() = instances;
    }

    Ok(())
}*/
