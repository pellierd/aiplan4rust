use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprNode, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::support::lang::{
    AtomSkeletonId, PredicateSymbolId, TypedList, TypedListId, TypedSymbol, VariableId,
};
use crate::analysis::reachability::datalog::core::{Atom, Term};
use crate::analysis::reachability::datalog::encoder::aliasing;
use crate::analysis::reachability::datalog::error::DatalogError;
use std::collections::HashMap;

/// Encodes a new auxiliary predicate based on a collection of atoms.
///
/// This is a helper function used during the transformation of complex formulas
/// (like AND/OR) into Horn clauses. It performs two main steps:
/// 1. It collects all unique variables from the provided `atoms` to determine
///    the signature (arity and types) of the new predicate.
/// 2. It allocates a new unique predicate ID and returns the corresponding [`Atom`].
///
/// # Arguments
/// * `atoms` - The list of atoms that will form the body (for AND) or the options (for OR)
///             of the rules associated with this auxiliary predicate.
/// * `parameters` - The typed list of parameters from the current scope (e.g., action parameters).
///
/// # Errors
/// Returns a [`DatalogError`] if variable collection fails or if there is an
/// inconsistency in the typed list.
///
pub(crate) fn allocate_auxiliary_predicate(
    atoms: &[Atom],
    parameters_id: TypedListId, // 🌟 Mis à jour : passage par ID
    next_aux_id: &mut usize,
    aux_defs: &mut Vec<AtomicFormulaSkeleton>,
    type_to_skeleton: &[AtomSkeletonId],
    current_aliases: &std::collections::HashMap<VariableId, Term>,
    store: &mut ExprStore,
) -> Result<(Atom, Vec<Atom>), DatalogError> {
    // 1. Collecte et résolution des variables
    let used_vars = extract_unique_variables(atoms, current_aliases)?;

    let mut resolved_terms: Vec<Term> = used_vars
        .into_iter()
        .map(|v_id| aliasing::resolve_var(v_id, current_aliases))
        .collect();

    resolved_terms.sort();
    resolved_terms.dedup();

    let final_vars: Vec<VariableId> = resolved_terms
        .iter()
        .filter_map(|t| {
            if let Term::Variable(v) = t {
                Some(*v)
            } else {
                None
            }
        })
        .collect();

    // --- 2. LA RÉPARATION (Type Guard Injection) ---
    let mut covered_vars = std::collections::HashSet::new();
    for atom in atoms {
        if !atom.is_negated() {
            for term in atom.terms() {
                if let Term::Variable(v) = term {
                    covered_vars.insert(*v);
                }
            }
        }
    }

    let mut secured_body = atoms.to_vec();

    // 🌟 Récupération locale et temporaire des paramètres pour injecter les Type Guards
    let parameters = store.fetch_typed_list(parameters_id)?;

    for v_id in &final_vars {
        if !covered_vars.contains(v_id) {
            let type_id = parameters[v_id.as_usize()].ty().members()[0].as_usize();
            let type_sk = type_to_skeleton[type_id];

            secured_body.push(Atom::new(type_sk, vec![Term::Variable(*v_id)]));
            covered_vars.insert(*v_id);
        }
    }

    // 3. Création de l'atome de tête
    let head = intern_auxiliary_signature(
        final_vars,
        resolved_terms,
        parameters_id, // 🌟 On relaie l'ID ici aussi
        next_aux_id,
        aux_defs,
        store,
    );

    Ok((head, secured_body))
}

/// Creates a new auxiliary atom and registers its skeleton locally.
///
/// This method is a support part of the **Skolemization** process during flattening.
/// It generates a unique predicate ID for a sub-formula and maps the provided
/// variables to their respective types based on the action's parameter list.
///
/// # Arguments
/// * `skeleton_vars` - The subset of variables that will become the terms of this auxiliary atom.
/// * `resolved_terms` - The final evaluated terms to pack into the resulting atom.
/// * `parameters` - The master list of typed variables from the current action/context
///   used to resolve the types of `skeleton_vars`.
/// * `store` - The global unique expressions arena used to intern the newly created signature.
///
/// # Returns
/// A new [`Atom`] configured with an auxiliary [`AtomSkeletonId`] and variable terms.
///
/// # Performance
/// - **ID Management**: Increments an internal counter in $O(1)$.
/// - **Type Resolution**: Direct $O(1)` lookup per variable using the `parameters` list.
/// - **Hash-Consing Allocation**: Interns `aux_params` into the `ExprStore`. If the signature
fn intern_auxiliary_signature(
    skeleton_vars: Vec<VariableId>,
    resolved_terms: Vec<Term>,
    parameters_id: TypedListId, // 🌟 Mis à jour : passage par ID
    next_aux_id: &mut usize,
    aux_defs: &mut Vec<AtomicFormulaSkeleton>,
    store: &mut ExprStore,
) -> Atom {
    let id = *next_aux_id;
    *next_aux_id += 1;

    // 🌟 Récupération temporaire de la liste originale pour lire les types
    let parameters = store
        .fetch_typed_list(parameters_id)
        .expect("Valid TypedListId");

    let mut aux_params = TypedList::new();
    for &v_id in &skeleton_vars {
        let ty = parameters[v_id.as_usize()].ty();
        aux_params.push(TypedSymbol::new(v_id, ty.clone()));
    }

    // --- On interne la liste brute dans l'arène globale ---
    let list_id = store.intern_typed_list(aux_params);

    // Enregistrement avec le TypedListId conforme au nouveau modèle
    aux_defs.push(AtomicFormulaSkeleton::new(
        PredicateSymbolId::from(id),
        list_id,
    ));

    Atom::new(AtomSkeletonId::from(id), resolved_terms)
}

/// Extracts a logical [`Atom`] from a specific expression node.
///
/// This function serves as the bridge between the high-level expression tree ([`Expr`][`ExprNode`][`Atom`]).
/// It resolves the predicate identity and maps each child argument to its concrete Datalog representation.
///
/// # Arguments
///
/// * `expr` - The global expression tree used to resolve the nature of child nodes.
/// * `node` - A reference to the current [`ExprNode`], which must represent an `AtomicFormula` or a `Comparison`.
///
/// # Returns
///
/// A [`Result`] containing the grounded or lifted [`Atom`], or a [`DatalogError`] if resolution fails.

pub(crate) fn extract_atom(node: ExprNode<'_>, store: &ExprStore) -> Result<Atom, DatalogError> {
    let kind = node.kind();
    let children = node.children();

    // 1. Fast determination of the Skeleton ID and the child skip offset.
    let (skeleton_id, skip_count) = match kind {
        ExprKind::Comparison(_) => (AtomSkeletonId::from(Atom::EQUALITY_ID), 0),
        ExprKind::AtomicFormula(sk_id) => (*sk_id, 1),
        _ => {
            return Err(DatalogError::incompatible_node(
                kind.clone(),
                node.id(), // 🌟 Plus précis : donne l'ID du nœud fautif directement
            ));
        }
    };

    // 2. Exact allocation to prevent vector resizing during the loop.
    let capacity = children.len().saturating_sub(skip_count);
    let mut terms = Vec::with_capacity(capacity);

    // 3. Optimized term collection.
    for &arg_id in children.iter().skip(skip_count) {
        // 🌟 Appel direct au store pour récupérer le nœud de l'argument
        let arg_node = store.fetch(arg_id)?;

        let term = match arg_node.kind() {
            ExprKind::Variable(v_id) => Term::Variable(*v_id),
            ExprKind::Object(obj_id) => Term::Constant(*obj_id),
            _ => {
                return Err(DatalogError::invalid_atom_argument(arg_id));
            }
        };
        terms.push(term);
    }

    Ok(Atom::new(skeleton_id, terms))
}

/// Collects all unique variables from a slice of atoms and returns them as a sorted vector.
///
/// This function identifies every `VariableId` present in the terms of the provided atoms
/// and produces a compact, deduplicated list.
///
/// # Logic and Implementation
/// This function uses a **Bitset** (a `u64` mask) to perform a "Union" operation of all
/// variables in a single pass.
/// 1. It iterates through all terms of all atoms.
/// 2. For each variable, it sets the corresponding bit in a 64-bit integer.
/// 3. It then extracts the set bits to reconstruct the `VariableId` list.
///
/// # Performance
/// - **Time Complexity**: $O(T + V)$, where $T$ is the total number of terms across all atoms
///   and $V$ is the number of unique variables. The bitwise extraction is extremely fast
///   thanks to CPU-level instructions.
/// - **Space Complexity**: $O(V)$ for the resulting `Vec`. The internal bitset resides
///   entirely on the stack (`u64`).
/// - **Instruction Level Optimization**:
///     - Uses `count_ones()` (POP_CNT) to pre-allocate the exact capacity of the `Vec`,
///       preventing reallocations.
///     - Uses `trailing_zeros()` (TZ_CNT) to jump directly to the next set bit, avoiding
///       a full 64-iteration loop.
///
/// # Constraints & Safety
/// - **Variable Limit**: This implementation is strictly limited to **64 variables**
///   (indexed 0 to 63). This is a design trade-off to ensure the bitset fits into
///   a single CPU register.
/// - **Debug Assertions**: In debug builds, the function will panic if a `VariableId`
///   exceeds `MAX_VARS`. In release builds, it uses a modulo wrap-around to prevent
///   undefined behavior, though this would indicate a logic error in the caller.
///
/// # Returns
/// A `Vec<VariableId>` sorted by ID in ascending order (due to the nature of bit-scanning).
fn extract_unique_variables(
    atoms: &[Atom],
    current_aliases: &HashMap<VariableId, Term>, // 💡 Ajouté ici pour propager
) -> Result<Vec<VariableId>, DatalogError> {
    // 💡 Transmis ici à local_collect_mask
    let mask = compute_variable_bitmask(atoms, current_aliases);
    Ok(decode_bitmask_to_variables(mask))
}

/// Version locale (associée) pour collecter le masque binaire des variables utilisées
fn compute_variable_bitmask(
    atoms: &[Atom],
    current_aliases: &HashMap<VariableId, Term>, // 💡 Type aligné sur ta fonction !
) -> u64 {
    let mut mask: u64 = 0;
    for atom in atoms {
        for term in atom.terms() {
            // AJOUT : Résolution systématique via ta fonction locale à 2 paramètres
            let resolved_term = match term {
                Term::Variable(v) => aliasing::resolve_var(*v, current_aliases),
                Term::Constant(_) => term.clone(),
            };

            if let Term::Variable(v) = resolved_term {
                mask |= 1 << v.as_usize();
            }
        }
    }
    mask
}

fn decode_bitmask_to_variables(mut mask: u64) -> Vec<VariableId> {
    let mut vars = Vec::with_capacity(mask.count_ones() as usize);
    while mask != 0 {
        let bit = mask.trailing_zeros();
        vars.push(VariableId::from(bit as usize));
        mask &= mask - 1; // 💡 Efface le bit de poids faible mis à 1
    }
    vars
}
