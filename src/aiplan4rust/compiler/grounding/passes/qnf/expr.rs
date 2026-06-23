use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprEvaluator;
use crate::aiplan4rust::compiler::grounding::binding::iter::BindingsIterator;
use crate::aiplan4rust::compiler::grounding::binding::{bind_with, BindingScratchpad};
use crate::aiplan4rust::compiler::grounding::error::GroundingError;
use crate::aiplan4rust::compiler::grounding::passes::qnf::ExpansionScratchpad;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprStore};
use crate::aiplan4rust::support::lang::TypedListId;

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

            let is_forall = matches!(entry_kind, ExprKind::ForallNew(_));

            let current_id = match entry_kind {
                ExprKind::ForallNew(vars) | ExprKind::ExistsNew(vars) => {
                    let body_id = body_id.expect("Quantifier must have a body node");
                    // On récupère la valeur expansée définitive depuis le cache
                    let expanded_body_id =
                        *expansion_scratchpad.cache.get(&body_id).unwrap_or(&body_id);

                    expand_quantified_node(
                        expanded_body_id,
                        vars,
                        is_forall,
                        store,
                        binding_scratchpad,
                        expansion_scratchpad,
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
/// Gère le produit cartésien, le tri des domaines et l'élagage dynamique (skip_at)
/// d'un nœud Forall ou Exists grâce au tracking de la variable coupable.
fn expand_quantified_node(
    body_id: ExprId,
    variables: TypedListId,
    is_forall: bool,
    store: &mut ExprStore,
    binding_scratchpad: &mut BindingScratchpad,
    expansion_scratchpad: &mut ExpansionScratchpad,
    value_registry: &ValueRegistry,
    evaluator: Option<&dyn ExprEvaluator>,
) -> Result<ExprId, GroundingError> {
    expansion_scratchpad.variables.clear();
    let vars_ref = store.fetch_typed_list(variables)?;
    expansion_scratchpad
        .variables
        .extend_from_slice(vars_ref.as_slice());

    // 2. L'itérateur pointe sur le buffer du scratchpad (qui est sur la pile / réutilisé)
    let variables = &expansion_scratchpad.variables;
    let mut iterator = BindingsIterator::new(variables, value_registry)?;

    let const_true = store.empty_and();
    let const_false = store.empty_or();

    if !iterator.has_next() && !variables.is_empty() {
        return Ok(if is_forall {
            store.empty_and() // ∀x ∈ ∅ => TRUE
        } else {
            store.empty_or() // ∃x ∈ ∅ => FALSE
        });
    }

    let alloc_capacity = std::cmp::min(iterator.total_count(), 64);
    let mut instances = Vec::with_capacity(alloc_capacity);

    while let Some(bindings) = iterator.next() {
        // 1. Appel à bind_with qui propage désormais le tuple (ExprId, Option<VariableId>)
        // On mappe l'erreur potentielle de Binding en GroundingError
        let (result_id, culprit) =
            bind_with(body_id, store, &bindings, evaluator, binding_scratchpad)?;

        // 2. CONSTANTES DOMINATRICES : Court-circuit absolu de la quantification
        // Forall + False => L'ensemble s'effondre immédiatement à False
        if is_forall && result_id == const_false {
            return Ok(const_false);
        }
        // Exists + True => L'ensemble s'effondre immédiatement à True
        if !is_forall && result_id == const_true {
            return Ok(const_true);
        }

        // 3. ÉLÉMENTS NEUTRES : Déclencheur du skip_at (Élagage combinatoire)
        // Forall + True  => N'apporte rien à l'intersection.
        // Exists + False => N'apporte rien à l'union.
        let is_neutral =
            (is_forall && result_id == const_true) || (!is_forall && result_id == const_false);

        if is_neutral {
            if let Some(var_id) = culprit {
                // On cherche l'index syntaxique de la variable coupable dans notre liste d'origine
                if let Some(syntax_idx) = variables.iter().position(|v| v.symbol() == var_id) {
                    // L'itérateur traduit cet index via sa permutation et sature le sous-arbre
                    iterator.skip_at(syntax_idx);
                    continue;
                }
            }
        }

        // Si ce n'est ni un court-circuit ni un élément neutre élagué, on conserve le nœud partiel
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
