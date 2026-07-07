use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::cause::Cause;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::database::Database;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::rule::Rule;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::compiler::lir::problem::ActionDef;
use crate::aiplan4rust::support::lang::{
    ActionSymbolId, AtomSkeletonId, TypeId, TypedList, VariableId,
};
use crate::analysis::inertia::table::InertiaTable;
use crate::analysis::reachability::datalog::encoder;
use crate::analysis::reachability::datalog::error::DatalogError;
use std::collections::HashMap;

/// Compile l'ensemble des définitions d'actions du domaine PDDL en règles Datalog logiques.
///
/// Cette fonction fusionnée parcourt chaque action, extrait sa signature, compile son corps
/// (préconditions), gère le cas des actions sans paramètres (bootstrap), et traduit ses effets.
pub(crate) fn encode_action_defs(
    rules: &mut Vec<Rule>,
    db: &mut Database,
    action_effects: &mut Vec<Vec<(Atom, Cause)>>,
    cache: &mut HashMap<Vec<Atom>, Atom>,
    aux_defs: &mut Vec<AtomicFormulaSkeleton>,
    next_aux_id: &mut usize,
    negation_offset: usize,
    action_defs: &[ActionDef],
    action_base_id: usize,
    inertia_table: &InertiaTable,
    type_to_skeleton: &[AtomSkeletonId],
    store: &mut ExprStore,
) -> Result<(), DatalogError> {
    for (id, action) in action_defs.iter().enumerate() {
        // Segmentation d'ID : calcul de l'ID du squelette Datalog pour cette action
        let action_sk_id = AtomSkeletonId::from(action_base_id + id);
        let action_index = action_sk_id.as_usize() - action_base_id;

        // A. Générer l'atome de nom (Pivot : action(?p1, ?p2...))
        let action_atom = encode_action_name(action, action_sk_id, store)?;

        // B. Générer la règle de déclenchement (Preconditions -> Action)
        encode_action_body(
            rules,
            cache,
            aux_defs,
            next_aux_id,
            action,
            action_atom.clone(),
            inertia_table,
            type_to_skeleton,
            store,
        )?;

        // --- LE BOOTSTRAP DE L'ACTION ---
        // Si l'action n'a aucun paramètre et un corps vide, elle est immédiatement applicable.
        let param_list_id = action.parameters();
        if let Some(trigger_rule) = rules.last() {
            if trigger_rule.body().is_empty() && store.fetch_typed_list(param_list_id)?.is_empty() {
                db.insert_delta_fact(action_sk_id, &[]);
            }
        }

        // C. Extraction et encodage des effets de l'action
        encoder::expr::encode_effects(
            action.effect(),
            &action_atom,
            param_list_id,
            action_index,
            rules,
            action_effects,
            cache,
            aux_defs,
            type_to_skeleton,
            next_aux_id,
            negation_offset,
            store,
        )?;
    }

    Ok(())
}

/// Version locale (associée) pour générer l'atome de nom de l'action
fn encode_action_name(
    action: &ActionDef,
    action_sk_id: AtomSkeletonId,
    store: &mut ExprStore,
) -> Result<Atom, DatalogError> {
    // 1. On récupère l'ID de la liste de paramètres
    let param_list_id = action.parameters();

    // 2. On extrait la liste concrète depuis le store du problème
    let parameters = store.fetch_typed_list(param_list_id)?;

    let head_terms: Vec<Term> = parameters
        .iter()
        .map(|param| Term::Variable(param.symbol()))
        .collect();

    Ok(Atom::new(action_sk_id, head_terms))
}

/// Version locale (associée) pour compiler le corps de l'action
fn encode_action_body(
    rules: &mut Vec<Rule>,
    cache: &mut HashMap<Vec<Atom>, Atom>,
    aux_defs: &mut Vec<AtomicFormulaSkeleton>,
    next_aux_id: &mut usize,
    action: &ActionDef,
    head: Atom,
    inertia_table: &InertiaTable,
    type_to_skeleton: &[AtomSkeletonId],
    store: &mut ExprStore,
) -> Result<(), DatalogError> {
    // 1. On récupère l'ID de la liste de paramètres
    let param_list_id = action.parameters();
    // 2. On extrait la liste concrète depuis le store

    // 💡 SÉCURITÉ BORROW CHECKER : On prend la taille avant d'emprunter immuablement via fetch_expr
    let store_len = store.len();

    // 💡 Appel mis à jour avec tous les nouveaux paramètres requis de la chaîne locale
    let precond_opt = encoder::expr::encode_preconditions(
        action.precondition(),
        action.parameters(),
        rules,
        cache,
        aux_defs,
        type_to_skeleton,
        next_aux_id,
        store,
    )?;

    // 2. On récupère les paramètres et on prépare l'ancre "intelligente"
    let mut final_action_body = Vec::new();
    let mut anchor_elements = Vec::new();
    let mut covered_vars = std::collections::HashSet::new();

    // ==========================================================
    // LE GARDIEN DE PARCOURS
    // ==========================================================
    let mut visited = vec![false; store_len];

    let precondition = store.fetch_expr(action.precondition())?;
    // Récupération des atomes en postorder
    let atoms = precondition
        .postorder()
        .references()
        .filter(|node| matches!(node.kind(), ExprKind::AtomicFormula(_)));

    for atom_node in atoms {
        let idx = atom_node.id().as_usize();
        if visited[idx] {
            continue;
        }
        visited[idx] = true;

        let skel_id = match atom_node.kind() {
            ExprKind::AtomicFormula(sk) => *sk,
            _ => unreachable!(),
        };

        let positive_id = skel_id.strip_negation();

        if inertia_table.is_predicate_positive_negative_inertia(positive_id)? {
            let children = atom_node.children();
            let mut terms = Vec::with_capacity(children.len().saturating_sub(1));

            for &term_id in children.iter().skip(1) {
                let term_node = precondition.fetch_node(term_id)?;

                let term = match term_node.kind() {
                    ExprKind::Variable(var_id) => {
                        let var_id = *var_id;
                        covered_vars.insert(var_id);
                        Term::Variable(var_id)
                    }
                    ExprKind::Object(obj_id) => Term::Constant(*obj_id),
                    _ => {
                        return Err(DatalogError::invalid_atom_argument(term_id));
                    }
                };
                terms.push(term);
            }

            let static_atom = Atom::new(skel_id, terms);
            anchor_elements.push(static_atom);
        }
    }

    // ==========================================================
    // TRAITEMENT DES PARAMÈTRES
    // ==========================================================
    // 🌟 Récupération locale des paramètres via l'ID et le store
    let parameters = store.fetch_typed_list(param_list_id)?;
    for (i, param) in parameters.iter().enumerate() {
        let var_id = VariableId::from(i);
        if !covered_vars.contains(&var_id) {
            let var_term = Term::Variable(var_id);
            let type_id = param.ty().members()[0].as_usize();
            let type_sk = type_to_skeleton[type_id];

            anchor_elements.push(Atom::new(type_sk, vec![var_term]));
            covered_vars.insert(var_id);
        }
    }

    // 4. Génération de l'Ancre et de la règle finale
    if !anchor_elements.is_empty() {
        // 💡 Alignement ici : on passe explicitement la référence mutable `next_aux_id`
        let anchor_head = create_anchor_atom(action.name(), parameters, &head, next_aux_id);

        rules.push(Rule::new(anchor_head.clone(), anchor_elements));
        final_action_body.push(anchor_head);
    }

    if let Some(p_atom) = precond_opt {
        final_action_body.push(p_atom);
    }

    rules.push(Rule::new(head, final_action_body));

    Ok(())
}

/// Version locale (associée) pour générer un atome d'ancre unique pour une action
fn create_anchor_atom(
    _action_id: ActionSymbolId, // Utile pour le debug/nommage futur
    _parameters: &TypedList<VariableId, TypeId>,
    action_head: &Atom,
    next_aux_id: &mut usize, // 💡 Remplace self.next_aux_id pour l'attribution de l'ID
) -> Atom {
    // 1. On alloue un nouvel ID auxiliaire via le compteur local
    // L'arité de l'ancre est exactement celle de l'action
    let anchor_id = *next_aux_id;
    *next_aux_id += 1;

    let sk_id = AtomSkeletonId::from(anchor_id);

    // 2. On récupère les termes (variables) de l'atome de tête.
    let terms = action_head.terms().to_vec();

    // 3. On crée l'atome d'ancre
    Atom::new(sk_id, terms)
}
