use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprNode, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::compiler::lir::problem::ActionDef;
use crate::aiplan4rust::support::lang::{
    AtomSkeletonId, ObjectId, PredicateSymbolId, TypeId, TypedSymbol,
};
use crate::analysis::reachability::datalog::context::DatalogContext;
use crate::analysis::reachability::datalog::error::DatalogError;
use crate::analysis::reachability::datalog::state::DatalogState;

/// Version locale (associée) pour initialiser les types sans bloquer `self`
pub(crate) fn declare_type_defs(
    type_defs: &[TypedSymbol<TypeId, TypeId>],
    type_to_skeleton: &mut Vec<AtomSkeletonId>,
    state: &mut DatalogState<'_>, // 🌟 Remplacement de current_id par le state global unifié
) {
    let num_types = type_defs.len();

    // Capacity for N domain types + 1 sentinel Root typing
    *type_to_skeleton = Vec::with_capacity(num_types + 1);

    // 1. Encode domain types first to ensure a 1:1 mapping with PDDL indices.
    for _ in 0..num_types {
        // 🌟 Utilisation et incrémentation via le compteur unique du state
        let sk_id = AtomSkeletonId::from(*state.next_aux_id);
        *state.next_aux_id += 1;

        type_to_skeleton.push(sk_id);
    }

    // 2. Encode the ROOT typing as a sentinel in the LAST slot.
    let root_type_sk = AtomSkeletonId::from(*state.next_aux_id);
    *state.next_aux_id += 1;

    type_to_skeleton.push(root_type_sk);
}

/// Parcourt et déclare l'ensemble des définitions d'actions du domaine.
///
/// Cette fonction alloue de manière séquentielle et monotone un ID Datalog unique
/// pour chaque action. Cela permet de représenter l'applicabilité des actions comme
/// des relations et d'assurer un décodage en O(1) sans table de hachage.
pub(crate) fn declare_action_defs(
    action_defs: &[ActionDef],
    state: &mut DatalogState<'_>, // 🌟 State en premier
) -> Result<(), DatalogError> {
    for action in action_defs {
        // 1. Récupération de l'ID de la liste de paramètres de l'action
        let list_id = action.parameters();

        // 3. Allocation monotone de l'ID d'ancre pour l'ID Segmentation via le state
        let anchor_id = *state.next_aux_id;
        *state.next_aux_id += 1;

        let predicate_id = PredicateSymbolId::from(anchor_id);

        // 4. Enregistrement direct dans le vecteur accumulateur du state
        state
            .aux_defs
            .push(AtomicFormulaSkeleton::new(predicate_id, list_id));
    }

    Ok(())
}

/// Version locale (associée) pour remplir la base de faits statiques à partir des objets
pub(crate) fn fill_db_from_objects(
    ctx: DatalogContext<'_>,      // 🌟 Regroupe type_to_skeleton
    state: &mut DatalogState<'_>, // 🌟 Regroupe db
    object_defs: &[TypedSymbol<ObjectId, TypeId>],
    type_defs: &[TypedSymbol<TypeId, TypeId>],
) -> Result<(), DatalogError> {
    // Récupération de la sentinelle ROOT depuis le context unifié
    let root_sk_id = *ctx
        .type_to_skeleton
        .last()
        .ok_or_else(|| DatalogError::internal_state("Root typing skeleton missing".to_string()))?;

    for object in object_defs {
        let obj_id = object.symbol();

        // 1. On l'insère dans la sentinelle ROOT via la db du state
        state.db.insert_stable_fact(root_sk_id, &[obj_id]);

        // 2. Pour chaque typing déclaré de l'objet (ex: [ball])
        for &type_id in object.ty() {
            // On l'insère dans le typing lui-même via le mapping du ctx
            let sk_id = ctx.type_to_skeleton[type_id.as_usize()];
            state.db.insert_stable_fact(sk_id, &[obj_id]);

            // 3. On l'insère dans TOUS les parents/membres identifiés par le flattener
            if let Some(ty_def) = type_defs.get(type_id.as_usize()) {
                for &parent_id in ty_def.ty().members() {
                    let parent_sk_id = ctx.type_to_skeleton[parent_id.as_usize()];
                    state.db.insert_stable_fact(parent_sk_id, &[obj_id]);
                }
            }
        }
    }
    Ok(())
}

/// Version locale (associée) pour ingérer l'état initial dans la base de faits delta
pub(crate) fn fill_db_from_init(
    state: &mut DatalogState<'_>, // 🌟 Regroupe db, injecté en premier
    init: ExprId,
    store: &mut ExprStore, // 🌟 Placé tout à la fin
) -> Result<(), DatalogError> {
    let mut iter = store.preorder(init);

    // VITESSE MAXIMALE : Tableau de booléens direct, indexé par la valeur numérique de l'ExprId.
    let mut visited = vec![false; store.len()];

    while let Some((id, _depth, entry)) = iter.next() {
        let idx = id.as_usize();

        if visited[idx] {
            continue;
        }
        visited[idx] = true;

        let node = ExprNode::new(id, entry);

        if let ExprKind::AtomicFormula(sk_id) = node.kind() {
            let sk_id = *sk_id;
            let children = node.children();

            // Allocation optimale pour les arguments réels
            let mut args = Vec::with_capacity(children.len().saturating_sub(1));

            for &arg_id in children.iter().skip(1) {
                let child_node = store.fetch(arg_id)?;

                if let ExprKind::Object(object_id) = child_node.kind() {
                    args.push(*object_id);
                } else {
                    return Err(DatalogError::invalid_atom_argument(arg_id));
                }
            }

            // Ingestion directe dans la DB Datalog via le state unifié
            state.db.insert_delta_fact(sk_id, &args);
        }
    }
    Ok(())
}
