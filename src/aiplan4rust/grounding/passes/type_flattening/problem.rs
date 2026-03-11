
use crate::aiplan4rust::lang::{Type, TypeId, TypedSymbol};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use std::collections::{HashMap, HashSet};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::grounding::passes::type_flattening::{atomic_formula_skeleton, atomic_function_skeleton, derived_predicate, expr, typed_symbol, task, action, method, initial_task_network};
use crate::aiplan4rust::tree::NodeId;
use crate::type_flattening::pivot_tracker::PivotTracker;

const EITHER_PREFIX: &str = "either";
const EITHER_SEP: &str = "_";

/// Flattens the problem's type hierarchy by replacing union types (`Either`)
/// with stable, atomic pivot types.
///
/// This transformation is a prerequisite for Grounding, as it eliminates
/// recursive or overlapping type definitions. It ensures that every named
/// type in the problem eventually points to a single, canonical "Pivot" type
/// representing a unique set of terminal leaf types.
///
/// # Arguments
///
/// * `problem` - A mutable reference to the [`LiftedProblem`] to be transformed.
///
/// # Returns
///
/// * `Ok(())` - If the flattening and remapping process succeeded.
/// * `Err(LirError)` - If a type resolution fails or an inconsistency is detected.
///
/// # Implementation Details
///
/// The process follows a 4-step pipeline designed for both structural correctness
/// and memory efficiency:
///
/// 1. **Hierarchy Resolution**: Flattens all `Either` definitions to their
///    base primitive roots (DFS-based).
/// 2. **Structural Mapping**: Uses a [`PivotTracker`] to identify unique
///    combinations of roots. Identical structures are mapped to the same Pivot ID.
/// 3. **Pivot Materialization**: Injects new pivot type definitions into the
///    problem's type table.
/// 4. **Re-binding**: Updates all original named types to become primitives
///    pointing to their respective pivots.
///
/// This implementation uses reusable buffers ([`Vec`] and [`String`]) to minimize
/// heap allocations during the traversal.
pub fn flatten(problem: &mut LiftedProblem) -> Result<(), LirError> {

    let num_original = problem.type_defs().len();
    let mut tracker = PivotTracker::new(num_original);

    //println!("{}", problem);
    flatten_type_def(problem, &mut tracker)?;
    //println!("{}", problem);

    let mut stack = Vec::with_capacity(num_original);
    flatten_problem_types(problem, &mut tracker, &mut stack)?;

    let mut name_buf = String::with_capacity(32);
    for (ty, _) in tracker.into_iter() {
        create_pivot(problem, &ty, &mut name_buf)?;
    }


    Ok(())
}

pub fn flatten_type_def(
    problem: &mut LiftedProblem,
    tracker: &mut PivotTracker
) -> Result<(), LirError> {
    let mut parent_set = HashSet::new();
    let mut stack = Vec::new();

    let num_initial_types = problem.type_defs().len();

    // We store planned changes to apply them only after the analysis is complete.
    // This prevents get_root_types from seeing a partially mutated graph.
    let mut mutations = Vec::with_capacity(num_initial_types);

    // --- PHASE 1: ANALYSIS (Immutable View) ---
    for i in 0..num_initial_types {
        let tid = TypeId::from(i);
        let ty = problem.type_defs()[i].ty();

        // 1. IDEMPOTENCE CHECK:
        // If the type already has exactly one member, it is already a primitive
        // (either a pivot or a redirection). We skip it to maintain stability.
        if ty.members().len() == 1 {
            continue;
        }

        // 2. ATOMIC ROOT:
        // Empty types are considered base atomic types. We ensure they point
        // to themselves as a primitive.
        if ty.is_empty() {
            mutations.push((i, tid));
            continue;
        }

        // 3. COMPLEX TYPE ANALYSIS (Either/Hierarchy):
        // Resolve the full hierarchy to find the underlying atomic roots.
        let roots = get_root_types(tid, problem, &mut parent_set, &mut stack)?;

        if roots.is_empty() {
            // Safety fallback for orphan types: redirect to itself.
            mutations.push((i, tid));
        } else if roots.len() == 1 {
            // OPTIMIZATION & SAFETY:
            // If a complex analysis results in a single root, we redirect directly
            // to that root without involving the tracker (preventing Type::either panics).
            mutations.push((i, roots[0]));
        } else {
            // MULTI-ROOT PIVOTING:
            // For true unions (2+ roots), we request a unique pivot ID from the tracker.
            // The tracker ensures that identical sets of roots share the same pivot.
            let pivot_id = tracker.next_type_id(&roots);
            mutations.push((i, pivot_id));
        }
    }

    // --- PHASE 2: APPLICATION (Mutation) ---
    // Apply all planned mutations to the problem's type definitions.
    for (index, target_id) in mutations {
        problem.type_defs_mut()[index].set_ty(Type::primitive(target_id));
    }

    Ok(())
}

/*pub fn flatten_type_def(problem: &mut LiftedProblem, tracker: &mut PivotTracker) -> Result<(), LirError> {
    let mut parent_set = HashSet::new();
    let mut stack = Vec::new();

    // --- PASSE 1 : Collecte des roots ---
    let num_initial_types = problem.type_defs().len();
    let mut resolved_roots = Vec::with_capacity(num_initial_types);
    for i in 0..num_initial_types {
        let tid = TypeId::from(i);
        let ty = problem.type_defs()[i].ty();

        // 1. Racine atomique
        if ty.is_empty() {
            resolved_roots.push(Some(Vec::new()));
            continue;
        }

        // 2. DÉJÀ APPLATI (Idempotence) :
        // Si le type a un seul membre, c'est soit un pivot (auto-réf),
        // soit une redirection déjà calculée (ex: t1 -> 3).
        // On met None pour dire à la Passe 3 : "ne touche à rien".
        if ty.members().len() == 1 {
            resolved_roots.push(None);
            continue;
        }

        // 3. À TRAITER (Either complexe)
        let roots = get_root_types(tid, problem, &mut parent_set, &mut stack)?;
        resolved_roots.push(Some(roots));
    }

    // --- PASSE 2 : ATTRIBUTION DES IDs ---
    for i in 0..num_initial_types {
        let tid = TypeId::from(i);
        let entry = &resolved_roots[i];

        match entry {
            None => {
                // Signal "Déjà fait" : on ne demande pas de modification

            }
            Some(roots) => {
                if roots.is_empty() {
                    // Racine atomique : reste elle-même
                    problem.type_defs_mut()[i].set_ty(Type::primitive(tid));
                } else {
                    // Nouveau pivot via tracker
                    let pivot_id = tracker.next_type_id(roots);
                    problem.type_defs_mut()[i].set_ty(Type::primitive(pivot_id));
                }
            }
        };
    }


    Ok(())
}*/

/// Resolves the full lineage of a type to its terminal primitive roots.
///
/// This function performs a Depth-First Search (DFS) to traverse the type hierarchy
/// and collect all leaf types (types with no members). It ensures that complex
/// nested `Either` types are flattened into a single-level list of base types.
///
/// # Arguments
///
/// * `ty_id` - The starting [`TypeId`] to resolve.
/// * `problem` - A reference to the [`LiftedProblem`] containing the type definitions.
/// * `lineage_set` - A reusable [`HashSet`] to collect unique root types without duplicates.
/// * `stack` - A reusable [`Vec`] used as a work stack for the DFS traversal.
///
/// # Returns
///
/// * `Ok(Vec<TypeId>)` - A sorted list where the first element is `ty_id` (the identity),
///   followed by all its terminal primitive roots.
/// * `Err(LirError)` - If a type in the hierarchy cannot be found.
///
/// # Implementation Details
///
/// The function follows "Option B" logic: it ignores intermediate `Either` types
/// and only collects types that have no further members (the "roots").
/// The resulting vector is normalized (sorted) to ensure deterministic signatures.
fn get_root_types(
    ty_id: TypeId,
    problem: &LiftedProblem,
    parents_set: &mut HashSet<TypeId>,
    stack: &mut Vec<(TypeId, Vec<TypeId>)>
) -> Result<Vec<TypeId>, LirError> {
    parents_set.clear();
    stack.clear();

    stack.push((ty_id, Vec::new()));

    while let Some((current, mut path)) = stack.pop() {
        let ty_def = problem.try_get_type(current)?.ty();

        // --- 1. DÉTECTION DE CYCLE ---
        if path.contains(&current) {
            // On autorise l'auto-référence des Pivots (marqueur d'aplatissement)
            if !ty_def.is_empty() && ty_def.members()[0] == current {
                continue;
            }
            panic!("ERREUR DOMAINE : Cycle détecté pour le type {:?}", current);
        }

        // --- 2. EXPLORATION ---
        if ty_def.is_empty() {
            if current != ty_id {
                parents_set.insert(current);
            }
        } else {
            // On descend dans les membres (soit un 'either', soit les racines d'un pivot)
            path.push(current);
            for &member in ty_def.members() {
                stack.push((member, path.clone()));
            }
        }
    }

    // --- 3. RÉSULTAT TRIÉ ---
    // On renvoie simplement la liste des racines atomiques trouvées.
    let mut result: Vec<TypeId> = parents_set.drain().collect();
    result.sort_unstable();

    Ok(result)
}

/// Materializes a new Pivot type within the problem's definition.
///
/// A Pivot is a canonical type that represents a unique combination of primitive roots.
/// Once created, this pivot acts as the single point of reference for all
/// original `Either` types that share the same leaf lineage.
///
/// # Arguments
///
/// * `problem` - A mutable reference to the [`LiftedProblem`] where the new type will be injected.
/// * `pivot` - The structural signature (roots) identified by the tracker.
/// * `pivot_name_buffer` - A reusable [`String`] buffer used by `reserve_pivot_id` to generate the pivot's name.
///
/// # Returns
///
/// * `Ok(TypeId)` - The unique identifier of the newly created Pivot type.
/// * `Err(LirError)` - If the name reservation or type definition injection fails.
///
/// # Implementation Details
///
/// The pivot's internal definition follows a specific layout: `[Self, Root1, Root2, ...]`.
/// This self-reference at index 0 is a requirement for the grounding engine to
/// treat the pivot as a valid member of its own type set.
fn create_pivot(
    problem: &mut LiftedProblem,
    pivot: &Type<TypeId>,
    pivot_name_buffer: &mut String,
) -> Result<TypeId, LirError> {
    // --- STEP 1: Identity Reservation ---
    // Generate a unique name (e.g., "either_a_b") and reserve a new TypeId
    // in the problem's symbol table.
    let pivot_id = reserve_pivot_id(problem, pivot, pivot_name_buffer)?;

    // --- STEP 2: Definition Construction ---
    // Prepare the member list. We allocate exactly pivot.len() + 1 to include
    // the pivot's self-reference at the head of the list.
    let mut members = Vec::with_capacity(pivot.len() + 1);
    members.push(pivot_id);
    members.extend(pivot.members());

    // --- STEP 3: Global Registration ---
    // Inject the new TypedSymbol into the problem's type definitions.
    // The pivot is defined as an 'Either' of its roots and itself.
    problem.add_type_defs(TypedSymbol::new(
        pivot_id,
        Type::either(members)
    ))?;

    Ok(pivot_id)
}

/// Generates a unique name for a pivot type and reserves its ID in the symbol table.
///
/// This function constructs a deterministic name based on the sorted names of its
/// parent roots (e.g., `either_airplane_truck`). It then interns this name to
/// obtain a stable symbol and registers it as a new type in the problem.
///
/// # Arguments
///
/// * `problem` - A mutable reference to the [`LiftedProblem`] to update the interner and symbol table.
/// * `pivot` - The structural signature ([`Type`] reference) containing the root TypeIds that define the pivot.
/// * `pivot_name_buffer` - A reusable [`String`] buffer to build the pivot's name without new allocations.
///
/// # Returns
///
/// * `Ok(TypeId)` - The newly reserved unique identifier for this pivot.
/// * `Err(LirError)` - If a parent TypeId cannot be resolved to a human-readable name.
///
/// # Implementation Details
///
/// 1. **Name Resolution**: Fetches the string representation of each root TypeId present in `pivot`.
/// 2. **Lexicographical Sorting**: Sorts parent names alphabetically to ensure that
///    `either_a_b` and `either_b_a` result in the exact same symbol.
/// 3. **String Construction**: Builds the name using the defined `EITHER_PREFIX` and `EITHER_SEP` in `pivot_name_buffer`.
/// 4. **Interning**: Converts the `String` into a `SymbolId` via the problem's interner.
/// 5. **Registration**: Adds the symbol to the problem's type registry to get a fresh [`TypeId`].
fn reserve_pivot_id(
    problem: &mut LiftedProblem,
    pivot: &Type<TypeId>,
    pivot_name_buffer: &mut String,
) -> Result<TypeId, LirError> {
    // Collect human-readable names of all roots.
    let mut parent_names: Vec<&str> = Vec::with_capacity(pivot.len());

    for id in pivot.iter() {
        let string_id = problem.type_symbols().try_get_ident(*id)?;
        let name = problem.interner().try_resolve_symbol(*string_id)?;
        parent_names.push(name);
    }

    // Sort names to guarantee name determinism (order-independent).
    parent_names.sort_unstable();

    // --- Name Construction ---
    // Reuse the provided buffer to avoid unnecessary heap allocations.
    pivot_name_buffer.clear();
    pivot_name_buffer.push_str(EITHER_PREFIX);
    pivot_name_buffer.push_str(EITHER_SEP);

    for (i, parent_name) in parent_names.iter().enumerate() {
        if i > 0 {
            pivot_name_buffer.push_str(EITHER_SEP);
        }
        pivot_name_buffer.push_str(parent_name);
    }

    // --- Symbol Interning ---
    // Convert the temporary String into a permanent, unique SymbolId.
    let name_id = problem.interner_mut().intern_symbol(pivot_name_buffer.clone());

    // --- ID Reservation ---
    // Register the symbol in the problem's type table and return the new TypeId.
    Ok(problem.add_type_symbol(name_id))
}

/// Traverses and updates all components of the [`LiftedProblem`] to use flattened types.
///
/// After the type definitions themselves have been flattened and pivots created,
/// this function propagates those changes throughout the entire problem structure.
/// It ensures that every reference to an original `Either` type is replaced by its
/// corresponding primitive pivot.
///
/// # Arguments
///
/// * `problem` - A mutable reference to the [`LiftedProblem`] to be updated.
/// * `tracker` - A reference to the [`PivotTracker`] containing the mapping
///   between original type signatures and their new Pivot IDs.
/// * `stack` - A reusable [`Vec<NodeId>`] buffer used for tree-walking during
///   expression flattening (prevents recursive allocations).
///
/// # Returns
///
/// * `Ok(())` - If all components were successfully remapped.
/// * `Err(LirError)` - If a type reference in any component cannot be resolved.
///
/// # Propagation Scope
///
/// This function handles the "Ripple Effect" of flattening across:
/// 1. **Objects & Constants**: Re-typing all physical entities in the problem.
/// 2. **Signatures**: Predicates, Functions, and HTN Task skeletons.
/// 3. **Logic**: Domain/Problem constraints, Goal conditions, and Derived Predicates.
/// 4. **Execution**: Action parameters/preconditions/effects and HTN Methods.
pub fn flatten_problem_types(
    problem: &mut LiftedProblem,
    tracker: &mut PivotTracker,
    stack: &mut Vec<NodeId>,
) -> Result<(), LirError> {

    // --- Objects & Constants ---
    // Re-type all objects (e.g., 'truck1' from 'truck' to 'pivot_truck_airplane').
    for object in problem.object_defs_mut() {
        typed_symbol::flatten_typed_object(object, tracker)?;
    }

    // --- Atomic Signatures ---
    // Update the parameter types for predicates and functions.
    for atomic_formula in problem.predicate_defs_mut() {
        atomic_formula_skeleton::flatten(atomic_formula, tracker)?;
    }

    for atomic_function in problem.function_defs_mut() {
        atomic_function_skeleton::flatten(atomic_function, tracker)?;
    }

    // --- Constraints & Global Logic ---
    // Perform a deep traversal of expression trees for domain and problem constraints.
    expr::flatten(problem.domain_constraints_mut(), tracker, stack)?;
    expr::flatten(problem.problem_constraints_mut(), tracker, stack)?;

    // --- HTN Tasks ---
    // Update the abstract task definitions in Hierarchical Task Networks.
    for task in problem.task_defs_mut() {
        task::flatten(task, tracker)?;
    }

    // --- Derived Predicates ---
    // Update axioms and their underlying logic.
    for derived_predicate in problem.derived_predicate_defs_mut() {
        derived_predicate::flatten(derived_predicate, tracker, stack)?;
    }

    // --- Actions ---
    // The core of the LIR: parameters, preconditions, and effects.
    for action in problem.action_defs_mut() {
        action::flatten(action, tracker, stack)?;
    }

    // --- HTN Methods ---
    // Update method preconditions and sub-task networks.
    for method in problem.method_defs_mut() {
        method::flatten(method, tracker, stack)?;
    }

    // --- Goal State ---
    // Final check on the goal expression tree.
    expr::flatten(problem.goal_mut(), tracker, stack)?;

    // --- Initial Task Network ---
    // Ensure the starting HTN state is consistent with the new types.
    initial_task_network::flatten(problem.initial_task_network_mut(), tracker)?;

    Ok(())
}
