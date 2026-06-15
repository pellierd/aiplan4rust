use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluator;
use crate::aiplan4rust::compiler::grounding::binding::iter::BindingsIterator;
use crate::aiplan4rust::compiler::grounding::binding::{bind_with, BindingScratchpad};
use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::qnf::ExpansionScratchpad;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprStore};
use crate::aiplan4rust::support::lang::{TypeId, TypedList, VariableId};

/// Point d'entrée standard autonome pour l'expansion des quantificateurs.
/// Consomme l'ancien ID et retourne le nouvel ID expansé.
pub fn expand(
    expr_id: ExprId,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
) -> Result<ExprId, GroundingError> {
    let mut binding_scratchpad = BindingScratchpad::new();
    let mut expansion_scratchpad = ExpansionScratchpad::new();
    expand_with(
        expr_id,
        store,
        value_registry,
        None,
        &mut binding_scratchpad,
        &mut expansion_scratchpad,
    )
}

/// Point d'entrée avancé avec pile explicite et zéro allocation à chaud.
/// Retourne le nouvel `ExprId` final après parcours post-ordre complet.
pub fn expand_with(
    expr_id: ExprId,
    store: &mut ExprStore,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
    binding_scratchpad: &mut BindingScratchpad,
    expansion_scratchpad: &mut ExpansionScratchpad,
) -> Result<ExprId, GroundingError> {
    expansion_scratchpad.clear();
    expansion_scratchpad.stack.push((expr_id, false));

    while let Some((old_id, children_pushed)) = expansion_scratchpad.stack.pop() {
        if !children_pushed {
            // --- ÉTAPE 1 : DESCENTE (Découverte) ---

            // Si ce nœud a déjà été complètement expansé par un autre chemin du DAG,
            // pas besoin de descendre dans ses enfants.
            if expansion_scratchpad.cache.contains_key(&old_id) {
                continue;
            }

            expansion_scratchpad.stack.push((old_id, true));

            let entry = &store[old_id];
            for &child_id in entry.children().iter().rev() {
                // On ne pousse sur la pile QUE si l'enfant n'est pas encore présent
                // avec sa valeur finale validée dans le cache.
                if !expansion_scratchpad.cache.contains_key(&child_id) {
                    expansion_scratchpad.stack.push((child_id, false));
                }
            }
        } else {
            // --- ÉTAPE 2 : REMONTÉE (Évaluation / Post-Order) ---

            // Double check de sécurité pour le DAG : si une autre branche a finalisé
            // ce nœud exact pendant qu'il attendait dans la pile, on skip.
            if expansion_scratchpad.cache.contains_key(&old_id) {
                continue;
            }

            let (entry_kind, body_id) = {
                let entry = &store[old_id];
                (entry.kind().clone(), entry.children().first().copied())
            };

            let is_forall = matches!(entry_kind, ExprKind::Forall(_));

            let current_id = match entry_kind {
                ExprKind::Forall(vars) | ExprKind::Exists(vars) => {
                    let body_id = body_id.expect("Quantifier must have a body node");
                    // On récupère la valeur expansée définitive depuis le cache
                    let expanded_body_id =
                        *expansion_scratchpad.cache.get(&body_id).unwrap_or(&body_id);

                    expand_quantified_node(
                        expanded_body_id,
                        &vars,
                        is_forall,
                        store,
                        binding_scratchpad,
                        value_registry,
                        evaluator,
                    )?
                }
                _ => {
                    let mut has_changed = false;
                    expansion_scratchpad.children_buffer.clear();

                    let entry = &store[old_id];
                    for &child_id in entry.children() {
                        let new_child_id = *expansion_scratchpad
                            .cache
                            .get(&child_id)
                            .unwrap_or(&child_id);
                        if new_child_id != child_id {
                            has_changed = true;
                        }
                        expansion_scratchpad.children_buffer.push(new_child_id);
                    }

                    if has_changed {
                        // Ton store.intern garantit le zero-allocation si le motif simplifié existe déjà
                        store.intern(entry_kind, &expansion_scratchpad.children_buffer)
                    } else {
                        old_id
                    }
                }
            };

            // L'écriture n'intervient qu'ici : elle marque la fin absolue du traitement de ce sous-arbre
            expansion_scratchpad.cache.insert(old_id, current_id);
        }
    }

    let final_root = *expansion_scratchpad.cache.get(&expr_id).unwrap_or(&expr_id);
    Ok(final_root)
}

/// Gère le produit cartésien et la réduction d'un nœud Forall ou Exists.
fn expand_quantified_node(
    body_id: ExprId,
    variables: &TypedList<VariableId, TypeId>,
    is_forall: bool,
    store: &mut ExprStore,
    binding_scratchpad: &mut BindingScratchpad,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
) -> Result<ExprId, GroundingError> {
    let mut iterator = BindingsIterator::new(variables, value_registry)?;

    if !iterator.has_next() && !variables.is_empty() {
        return Ok(if is_forall {
            store.empty_and() // ∀x ∈ ∅ => TRUE
        } else {
            store.empty_or() // ∃x ∈ ∅ => FALSE
        });
    }

    let mut instances = Vec::with_capacity(8);

    while let Some(bindings) = iterator.next() {
        let result_id = bind_with(body_id, store, &bindings, evaluator, binding_scratchpad)?;

        // Court-circuit immédiat si une constante dominatrice est rencontrée
        if result_id == store.empty_or() {
            if is_forall {
                return Ok(store.empty_or());
            }
            continue;
        }

        if result_id == store.empty_and() {
            if !is_forall {
                return Ok(store.empty_and());
            }
            continue;
        }

        instances.push(result_id);
    }

    // --- AGREGATION STRUCTURELLE FINALE ---
    if instances.is_empty() {
        Ok(if is_forall {
            store.empty_and()
        } else {
            store.empty_or()
        })
    } else {
        let new_kind = if is_forall {
            ExprKind::And
        } else {
            ExprKind::Or
        };
        Ok(store.intern(new_kind, &instances))
    }
}
