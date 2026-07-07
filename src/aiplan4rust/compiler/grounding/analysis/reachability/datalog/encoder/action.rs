use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::cause::Cause;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::rule::Rule;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::ActionDef;
use crate::aiplan4rust::support::lang::{
    ActionSymbolId, AtomSkeletonId, TypeId, TypedList, VariableId,
};
use crate::analysis::reachability::datalog::context::DatalogContext;
use crate::analysis::reachability::datalog::core::atom::AtomArgs;
use crate::analysis::reachability::datalog::core::rule::RuleBody;
use crate::analysis::reachability::datalog::encoder;
use crate::analysis::reachability::datalog::error::DatalogError;
use crate::analysis::reachability::datalog::state::DatalogState;

/// Compile l'ensemble des définitions d'actions du domaine PDDL en règles Datalog logiques.
///
/// Cette fonction fusionnée parcourt chaque action, extrait sa signature, compile son corps
/// (préconditions), gère le cas des actions sans paramètres (bootstrap), et traduit ses effets.
pub(crate) fn encode_action_defs(
    ctx: DatalogContext<'_>,
    state: &mut DatalogState<'_>,
    action_effects: &mut Vec<Vec<(Atom, Cause)>>,
    action_defs: &[ActionDef],
    action_base_id: usize,
    store: &mut ExprStore,
) -> Result<(), DatalogError> {
    for (id, action) in action_defs.iter().enumerate() {
        // Segmentation d'ID : calcul de l'ID du squelette Datalog pour cette action
        let action_sk_id = AtomSkeletonId::from(action_base_id + id);
        let action_index = action_sk_id.as_usize() - action_base_id;

        // A. Générer l'atome de nom (Pivot : action(?p1, ?p2...))
        let action_atom = encode_action_name(action, action_sk_id, store)?;

        // 🌟 Mise à jour locale du contexte avec les paramètres de l'action courante
        let action_ctx = DatalogContext {
            param_list_id: action.parameters(),
            ..ctx
        };

        // B. Générer la règle de déclenchement (Preconditions -> Action)
        encode_action_body(action_ctx, state, action, action_atom.clone(), store)?;

        // --- LE BOOTSTRAP DE L'ACTION ---
        // Si l'action n'a aucun paramètre et un corps vide, elle est immédiatement applicable.
        let param_list_id = action.parameters();
        if let Some(trigger_rule) = state.rules.last() {
            if trigger_rule.body().is_empty() && store.fetch_typed_list(param_list_id)?.is_empty() {
                state.db.insert_delta_fact(action_sk_id, &[]);
            }
        }

        // C. Extraction et encodage des effets de l'action
        // 🌟 Appel mis à jour avec le contexte localisé et l'état complet unifié
        encoder::expr::encode_effects(
            action.effect(),
            &action_atom,
            action_index,
            action_ctx,
            state,
            action_effects,
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

    // 🚀 OPTIMISATION : Accumulation directe dans le SmallVec natif de l'Atom
    let mut head_terms = AtomArgs::with_capacity(parameters.len());

    for param in parameters.iter() {
        head_terms.push(Term::Variable(param.symbol()));
    }

    // Utilisation du nouveau constructeur n-aire sans transit par la heap
    Ok(Atom::nary(action_sk_id, head_terms))
}

/// Version locale (associée) pour compiler le corps de l'action
fn encode_action_body(
    context: DatalogContext<'_>,
    state: &mut DatalogState<'_>,
    action: &ActionDef,
    head: Atom,
    store: &mut ExprStore,
) -> Result<(), DatalogError> {
    // 1. On récupère l'ID de la liste de paramètres
    let param_list_id = action.parameters();

    // 💡 SÉCURITÉ BORROW CHECKER : On prend la taille avant d'emprunter immuablement via fetch_expr
    let store_len = store.len();

    // 🌟 On met à jour le contexte pour les préconditions de cette action spécifique
    let precond_ctx = DatalogContext {
        param_list_id,
        ..context
    };

    // 💡 Appel mis à jour avec le contexte et l'état unifiés
    let precond_opt =
        encoder::expr::encode_preconditions(action.precondition(), precond_ctx, state, store)?;

    // 2. On récupère les paramètres et on prépare l'ancre "intelligente"
    let mut final_action_body = RuleBody::new();
    let mut anchor_elements = RuleBody::new();
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

        if context
            .inertia_table
            .is_predicate_positive_negative_inertia(positive_id)?
        {
            let children = atom_node.children();

            // 🚀 OPTIMISATION : Accumulateur direct sur la pile via SmallVec
            let mut terms = AtomArgs::with_capacity(children.len().saturating_sub(1));

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

            // 🚀 OPTIMISATION : Utilisation du constructeur n-aire sans allocation de Vec intermédiaire
            let static_atom = Atom::nary(skel_id, terms);
            anchor_elements.push(static_atom);
        }
    }

    // ==========================================================
    // TRAITEMENT DES PARAMÈTRES
    // ==========================================================
    // 🌟 Récupération locale des paramètres via l'ID et le store
    let parameters = store.fetch_typed_list(param_list_id)?;

    // 🚀 OPTIMISATION BONUS : Utilisation d'un bitmask u64 au lieu d'un HashSet
    let mut covered_mask: u64 = 0;

    for (i, param) in parameters.iter().enumerate() {
        let var_id = VariableId::from(i);
        let bit_projected = 1 << i;

        // Si la variable n'est pas encore couverte (bit à 0)
        if (covered_mask & bit_projected) == 0 {
            let var_term = Term::Variable(var_id);
            let type_id = param.ty().members()[0].as_usize();
            let type_sk = context.type_to_skeleton[type_id];

            // 🚀 OPTIMISATION : Utilisation de Atom::unary au lieu de Atom::new + vec![]
            // Aucun vecteur n'est alloué sur le tas ici.
            anchor_elements.push(Atom::unary(type_sk, var_term));

            // On marque la variable comme couverte dans le bitmask
            covered_mask |= bit_projected;
        }
    }

    // 4. Génération de l'Ancre et de la règle finale
    if !anchor_elements.is_empty() {
        // 💡 Alignement ici : on passe explicitement la référence mutable `next_aux_id`
        let anchor_head = create_anchor_atom(action.name(), parameters, &head, state.next_aux_id);

        state
            .rules
            .push(Rule::new(anchor_head.clone(), anchor_elements));
        final_action_body.push(anchor_head);
    }

    if let Some(p_atom) = precond_opt {
        final_action_body.push(p_atom);
    }

    state.rules.push(Rule::new(head, final_action_body));

    Ok(())
}

fn create_anchor_atom(
    _action_id: ActionSymbolId, // Utile pour le debug/nommage futur
    _parameters: &TypedList<VariableId, TypeId>,
    action_head: &Atom,
    next_aux_id: &mut usize,
) -> Atom {
    // 1. On alloue un nouvel ID auxiliaire via le compteur local
    let anchor_id = *next_aux_id;
    *next_aux_id += 1;

    let sk_id = AtomSkeletonId::from(anchor_id);

    // 2. 🚀 OPTIMISATION : On extrait et clone les termes directement dans un SmallVec
    let terms = AtomArgs::from_slice(action_head.arguments());

    // 3. On crée l'atome d'ancre via le constructeur n-aire
    Atom::nary(sk_id, terms)
}
