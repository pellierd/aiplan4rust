
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::grounding::engine::{GroundingEngine, Substitution};
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::iterator::DomainIterator;
use crate::aiplan4rust::lang::{TypeId, TypedList, VariableId};
use crate::aiplan4rust::lir::expr::{Expr, ExprContent, ExprKind};
use crate::aiplan4rust::tree::NodeId;


pub fn expand(
    expr: &mut Expr,
    engine: &GroundingEngine,
) -> Result<(), GroundingError> {
    let mut global_change = false;

    // L'astuce : On ne collecte que les IDs des nœuds Forall et Exists
    // Mais attention : en post-ordre pour respecter l'imbrication !
    // 1. Collecte des IDs en post-ordre pour traiter les imbrications de l'intérieur vers l'extérieur
    let quantifier_ids: Vec<NodeId> = expr.postorder()
        .ids()
        .filter(|(_, node)| is_quantifier(node.kind()))
        .map(|(id, _)| id)
        .collect();

    if quantifier_ids.is_empty() {
        return Ok(());
    }

    for node_id in quantifier_ids {
        // 1. On récupère le nœud (sécurité arène)
        let node = match expr.try_node(node_id) {
            Ok(n) => n,
            Err(_) => continue,
        };

        // 2. On vérifie qu'il est TOUJOURS un quantificateur
        // S'il a été simplifié par un enfant (en post-order), on le saute
        if !is_quantifier(node.kind()) {
            continue;
        }

        if expand_quantified_expr(expr, node_id, engine)? {
            engine.logic_engine().simplify_from(expr, node_id)?;
            global_change = true;
        }
}

    if global_change {
        engine.logic_engine().simplify(expr)?;
    }

    Ok(())
}

/// Helper pour identifier les quantificateurs.
fn is_quantifier(kind: ExprKind) -> bool {
    matches!(kind, ExprKind::Forall | ExprKind::Exists)
}

pub fn expand_quantified_expr(
    expr: &mut Expr,
    node_id: NodeId,
    engine: &GroundingEngine,
) -> Result<bool, GroundingError> {
    // --- 1. EXTRACTION ---
    let (variables, body_id, is_forall) = {
        let node = expr.try_node(node_id)?;
        let vars = node.content().try_quantifier_vars()?.clone();
        let body = node.try_child(0)?;
        let forall = node.kind() == ExprKind::Forall;
        (vars, body, forall)
    };

    // --- 2. RÉCUPÉRATION DES DOMAINES ---
    let var_domains = engine.registry().get_variable_domains(&variables);
    let mut iterator = DomainIterator::new(var_domains)?;

    if handle_empty_domains(expr, node_id, &variables, is_forall, iterator.has_next())? {
        return Ok(true);
    }

    // --- 3. GÉNÉRATION ---
    let mut instances = Vec::new();
    let mut substitution = Substitution::with_capacity(variables.len());

    while let Some(combo) = iterator.next() {
        substitution.clear();
        for (i, &obj_id) in combo.iter().enumerate() {
            substitution.insert(variables[i].symbol(), obj_id);
        }

        // Appel de la fonction de clonage qui renvoie un NodeId (simplifié)
        let result_id = engine.instantiate_in_place(expr, body_id, &substitution)?;
        let body_node = expr.try_node(result_id)?;
        // --- DÉTECTION DES CONSTANTES VIA L'ARÈNE ---
        if body_node.is_empty_or() { // Représente FALSE
            if is_forall {
                // FORALL + un seul FALSE = FALSE GLOBAL
                expr.set_to_bool(node_id, false)?;
                return Ok(true);
            }
            // Si c'est un Exists, on ignore juste cette branche False
            continue;
        }

        if body_node.is_empty_and() { // Représente TRUE
            if !is_forall {
                // EXISTS + un seul TRUE = TRUE GLOBAL
                expr.set_to_bool(node_id, true)?;
                return Ok(true);
            }
            // Si c'est un Forall, on ignore cette branche True (neutre)
            continue;
        }

        // Si ce n'est pas une constante vide, c'est une instance dynamique
        instances.push(result_id);
    }

    // --- 4. FINALISATION ---
    if instances.is_empty() {
        // Si tout a été filtré (ex: Forall où tout est True), le résultat est la valeur neutre
        expr.set_to_bool(node_id, is_forall)?;
    } else {
        let new_kind = if is_forall { ExprKind::And } else { ExprKind::Or };
        let node_mut = expr.try_node_mut(node_id)?;
        node_mut.set_kind(new_kind);
        node_mut.set_content(ExprContent::None);
        *node_mut.children_mut() = instances;

        // Simplification finale du parent
        engine.logic_engine().simplify_from(expr, node_id)?;
    }

    Ok(true)
}



/// Gère les cas limites des domaines vides pour les quantificateurs.
///
/// Selon la logique du premier ordre :
/// - ∀x ∈ ∅, P(x) est TRUE (Vacuous Truth)
/// - ∃x ∈ ∅, P(x) est FALSE
fn handle_empty_domains(
    expr: &mut Expr,
    node_id: NodeId,
    variables: &TypedList<VariableId, TypeId>,
    is_forall: bool,
    has_next: bool,
) -> Result<bool, GroundingError> {
    // Si l'itérateur n'a pas de combinaisons alors que des variables sont définies,
    // c'est qu'au moins un des domaines est vide.
    if !has_next && !variables.is_empty() {
        // Pour Forall -> True, pour Exists -> False
        expr.set_to_bool(node_id, is_forall)?;
        return Ok(true); // Indique qu'un changement (élagage) a été fait
    }
    Ok(false)
}
