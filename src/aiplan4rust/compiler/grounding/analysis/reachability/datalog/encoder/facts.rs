use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind, ExprNode, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::compiler::lir::problem::ActionDef;
use crate::aiplan4rust::support::lang::{
    AtomSkeletonId, ObjectId, PredicateSymbolId, Type, TypeId, TypedList, TypedSymbol, VariableId,
};
use crate::analysis::reachability::datalog::database::Database;
use crate::analysis::reachability::datalog::error::DatalogError;

/// Encodes a PDDL Type as a unary Datalog predicate and maintains a semantic mapping.
/// Encodes a PDDL Type as a unary Datalog predicate using sequential allocation.
///
/// This function is a support component of the **ID Segmentation** strategy. Type IDs
/// are allocated contiguously, enabling O(1) conversion between Datalog
/// `AtomSkeletonId` and PDDL `TypeId` through pointer-free arithmetic.
///
/// # Returns
///
/// The unique [`AtomSkeletonId`] representing this typing.
/// The mapping to the original `TypeId` is implicit:
/// `TypeId = sk_id - fluence_threshold`.
///
/// # Process
///
/// 1. **Monotonic Allocation**: Uses the `next_aux_id` counter to ensure the ID
///    falls within the reserved segment for types.
/// 2. **Signature Definition**: Creates a unary predicate schema `(type_name ?v0)`.
///    The argument `?v0` uses `Type::root()` because this predicate itself
///    defines the domain membership for objects.
/// 3. **Schema Consistency**: Registers the skeleton in `aux_defs` to allow the
///    rule compiler to verify predicate arity (always 1 for types).

#[inline]
pub(crate) fn declare_type(
    store: &mut ExprStore,
    next_aux_id: &mut usize,
    aux_defs: &mut Vec<AtomicFormulaSkeleton>,
) -> AtomSkeletonId {
    // On relaie simplement les arguments à la nouvelle version libre
    declare_auxiliary_predicate(1, None, store, next_aux_id, aux_defs)
}

/// Crée un prédicat auxiliaire de manière flexible (sans structure self).
/// - Si `types` est `Some`: utilise la liste fournie (zéro boucle inutile).
/// - Si `types` est `None`: génère une signature générique de taille `arity`.
fn declare_auxiliary_predicate(
    arity: usize,
    types: Option<TypedList<VariableId, TypeId>>,
    store: &mut ExprStore,
    next_aux_id: &mut usize,
    aux_defs: &mut Vec<AtomicFormulaSkeleton>,
) -> AtomSkeletonId {
    let id = *next_aux_id;
    *next_aux_id += 1;

    let sk_id = AtomSkeletonId::from(id);
    let predicate_id = PredicateSymbolId::from(id);

    let raw_parameters = match types {
        // Cas 1 : On a déjà les types (ex: une Action)
        Some(p) => p,
        // Cas 2 : On doit générer des types root (ex: une Union ou un AND)
        None => {
            let mut arguments = TypedList::new();
            for i in 0..arity {
                arguments.push(TypedSymbol::new(VariableId::from(i), Type::root()));
            }
            arguments
        }
    };

    // Enregistrement unique dans le store d'expressions fourni
    let list_id = store.intern_typed_list(raw_parameters);

    aux_defs.push(AtomicFormulaSkeleton::new(predicate_id, list_id));

    sk_id
}

/// Version locale (associée) pour initialiser les types sans bloquer `self`
pub(crate) fn declare_type_defs(
    type_defs: &[TypedSymbol<TypeId, TypeId>],
    type_to_skeleton: &mut Vec<AtomSkeletonId>,
    current_id: &mut usize, // 💡 On passe le compteur d'IDs qui remplace l'état de self
) {
    let num_types = type_defs.len();

    // Capacity for N domain types + 1 sentinel Root typing
    *type_to_skeleton = Vec::with_capacity(num_types + 1);

    // 1. Encode domain types first to ensure a 1:1 mapping with PDDL indices.
    for _ in 0..num_types {
        // 💡 Appel à la logique locale d'encodage (génération de l'ID + incrément)
        let sk_id = AtomSkeletonId::from(*current_id);
        *current_id += 1;

        type_to_skeleton.push(sk_id);
    }

    // 2. Encode the ROOT typing as a sentinel in the LAST slot.
    let root_type_sk = AtomSkeletonId::from(*current_id);
    *current_id += 1;

    type_to_skeleton.push(root_type_sk);
}

/// Parcourt et déclare l'ensemble des définitions d'actions du domaine.
///
/// Cette fonction alloue de manière séquentielle et monotone un ID Datalog unique
/// pour chaque action. Cela permet de représenter l'applicabilité des actions comme
/// des relations et d'assurer un décodage en $O(1)$ sans table de hachage.
pub(crate) fn declare_action_defs(
    action_defs: &[ActionDef],
    store: &mut ExprStore,
    current_id: &mut usize,
    aux_defs: &mut Vec<AtomicFormulaSkeleton>,
) -> Result<(), DatalogError> {
    for action in action_defs {
        // 1. Récupération de l'ID de la liste de paramètres de l'action
        let list_id = action.parameters();

        // 2. Vérification de la validité de l'arité depuis le store d'expressions
        let _arity = store.typed_list_len(list_id)?;

        // 3. Allocation monotone de l'ID d'ancre pour l'ID Segmentation
        let anchor_id = *current_id;
        *current_id += 1;

        let predicate_id = PredicateSymbolId::from(anchor_id);

        // 4. Enregistrement du squelette pour maintenir la cohérence du schéma LIR
        aux_defs.push(AtomicFormulaSkeleton::new(predicate_id, list_id));
    }

    Ok(())
}

/// Version locale (associée) pour remplir la base de faits statiques à partir des objets
pub(crate) fn fill_db_from_objects(
    db: &mut Database,                   // 💡 Injecté au lieu de self.db
    type_to_skeleton: &[AtomSkeletonId], // 💡 Injecté au lieu de self.type_to_skeleton
    object_defs: &[TypedSymbol<ObjectId, TypeId>],
    type_defs: &[TypedSymbol<TypeId, TypeId>],
) -> Result<(), DatalogError> {
    // Récupération de la sentinelle ROOT depuis le tableau local injecté
    let root_sk_id = *type_to_skeleton
        .last()
        .ok_or_else(|| DatalogError::internal_state("Root typing skeleton missing".to_string()))?;

    for object in object_defs {
        let obj_id = object.symbol();

        // 1. On l'insère dans la sentinelle ROOT (le garde-fou universel) via la db locale
        db.insert_stable_fact(root_sk_id, &[obj_id]);

        // 2. Pour chaque typing déclaré de l'objet (ex: [ball])
        for &type_id in object.ty() {
            // On l'insère dans le typing lui-même
            let sk_id = type_to_skeleton[type_id.as_usize()];
            db.insert_stable_fact(sk_id, &[obj_id]);

            // 3. On l'insère dans TOUS les parents/membres identifiés par le flattener
            if let Some(ty_def) = type_defs.get(type_id.as_usize()) {
                for &parent_id in ty_def.ty().members() {
                    let parent_sk_id = type_to_skeleton[parent_id.as_usize()];
                    db.insert_stable_fact(parent_sk_id, &[obj_id]);
                }
            }
        }
    }
    Ok(())
}

/// Version locale (associée) pour ingérer l'état initial dans la base de faits delta
pub(crate) fn fill_db_from_init(
    db: &mut Database, // 💡 Injecté au lieu de self.db
    init: ExprId,
    store: &mut ExprStore,
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

            // Ingestion directe dans la DB Datalog locale passée en argument
            db.insert_delta_fact(sk_id, &args);
        }
    }
    Ok(())
}
