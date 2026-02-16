//! Type Flattening and Canonicalization for Lifted Problems.
//!
//! This module provides the infrastructure to transform a complex PDDL/HDL type
//! hierarchy containing union types (`Type::Either`) into a flat, primitive-only
//! representation.
//!
//! # Overview
//!
//! Many downstream processes, particularly **Grounding**, struggle with the
//! recursive nature of `Either` types. This module eliminates that complexity by:
//! 1.  Performing a DFS traversal to find terminal "leaf" types.
//! 2.  Creating unique "Pivot" types that represent specific sets of leaves.
//! 3.  Updating the entire problem to use these stable Pivots.
//!
//! # Architecture
//!
//! The module is organized as a dispatcher that propagates type changes across
//! all LIR (Lifted Intermediate Representation) components:
//! - **Definitions**: Handled in the module root via pivot maps.
//! - **Logic**: Propagated to `expr`, `action`, `method`, etc.
//! - **Symbols**: Managed through `typed_symbol` and `typed_list`.
//!
//! # Naming Convention
//!
//! Flattened types are automatically named using the format:
//! `{EITHER_PREFIX}{EITHER_SEP}{type1}{EITHER_SEP}{type2}...`
//! (e.g., `either_truck_airplane`).

use crate::aiplan4rust::lang::{Type, TypeID, TypedSymbol};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use std::collections::{HashMap, HashSet, VecDeque};
use std::collections::hash_map::Entry;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::grounding::passes::types::{atomic_formula_skeleton, atomic_function_skeleton, derived_predicate, expr, typed_symbol, task, action, method, initial_task_network};
use crate::aiplan4rust::grounding::value_domain::ValueDomain;

const EITHER_PREFIX: &str = "either";
const EITHER_SEP: &str = "_";

/// Flattens the problem's type hierarchy by replacing union types (`Either`)
/// with atomic pivot types.
///
/// This process transforms a complex typed problem into a "flat" representation
/// required for subsequent grounding phases. The operation is performed
/// in three major steps:
///
/// 1. **Analysis & Creation**: Identifies all `Either` combinations, resolves
///    their terminal root parents, and generates unique "pivot" types in the domain.
/// 2. **Type Definition Mutation**: Updates the problem's type table so that
///    original `Either` types now point to these pivots as primitive types.
/// 3. **Component Propagation**: Traverses the entire problem (actions, methods,
///    goals, objects) to update every type reference to the new flattened structure.
///
/// # Parameters
/// * `problem` - A mutable reference to the [`LiftedProblem`] to be transformed.
///
/// # Returns
/// * `Ok(())` if the flattening and remapping process succeeded.
/// * `Err(LirError)` if an error occurs (e.g., type resolution failure or interner error).
///
/// # Example
/// ```rust
/// use crate::aiplan4rust::lir::problem::types;
///
/// // Given a problem with: type truck = either(heavy, light)
/// types::flatten(&mut problem)?;
/// // After the call, 'truck' is a primitive type pointing to a unique pivot.
/// ```
pub fn flatten(problem: &mut LiftedProblem) -> Result<(), LirError> {
    // Step 1: Create the mapping and inject new pivot types
    // Analyzes the hierarchy to ensure each type combination is unique and canonical.
    let flatten_types_map = flatten_types_def(problem)?;

    // Step 2: Propagate changes to the rest of the problem components
    // Updates action signatures, method parameters, and object definitions.
    apply_map_to_problem_components(problem, &flatten_types_map)?;

    Ok(())
}


/*/// Construit un mapping de TypeID vers ValueDomain.
/// Cette fonction doit être appelée APRÈS 'flatten' pour garantir que
/// les objets pointent vers des types atomiques/pivots.
pub fn build_value_domains(problem: &LiftedProblem) -> Vec<ValueDomain> {
    // 1. Pré-allocation du vecteur
    // On utilise la taille de la table des types pour que chaque TypeID soit un index valide.
    let n_types = problem.type_defs().len();
    let mut domains = vec![ValueDomain::new(); n_types];

    // 2. Parcours de tous les objets (Constants + Problem Objects)
    for obj_def in problem.object_defs() {
        let obj_id = obj_def.symbol();

        // On récupère le TypeID du Pivot (le premier membre après flatten)
        if let Some(&type_id) = obj_def.ty().members().first() {
            let idx = type_id.as_usize();

            // A. Ajout au type direct (le Pivot)@
            domains[idx].add_object(obj_id);

            // B. Propagation aux parents (Héritage)
            if let Ok(ts) = problem.try_get_type(type_id) {
                let ty_def = ts.ty();

                // Si ty() est vide, c'est une racine -> rien à faire.
                // Sinon, c'est un pivot -> on propage aux membres racines.
                if !ty_def.is_empty() {
                    for &parent_id in ty_def.members() {
                        let p_idx = parent_id.as_usize();
                        domains[p_idx].add_object(obj_id);
                    }
                }
            }
        }
    }

    domains
}*/

/// STEP 3: Propagate Type Changes to Problem Components
///
/// Traverses all components of the `LiftedProblem` and updates their internal
/// type references using the provided mapping. This ensures that every
/// signature, variable, and object now refers to the flattened pivot types.
///
/// This function acts as a dispatcher, calling specific flattening logic for:
/// - Objects and Constants
/// - Predicate and Function Skeletons
/// - Actions (Instantaneous and Durative)
/// - HTN structures (Tasks, Methods, and ITN)
/// - Logical Expressions (Goals and Constraints)
///
/// # Arguments
/// * `problem` - The [`LiftedProblem`] to be updated.
/// * `map` - The pre-calculated mapping from `Either` structures to Pivot `TypeID`s.
fn apply_map_to_problem_components(
    problem: &mut LiftedProblem,
    map: &HashMap<Type<TypeID>, TypeID>
) -> Result<(), LirError> {
    // Note: We use the 'map' passed as an argument to maintain consistency
    // with the changes already applied to the type definitions.

    // Update Objects
    for object in problem.object_defs_mut() {
        typed_symbol::flatten_typed_object(object, map)?;
    }

    // Update Predicate and Function signatures
    for atomic_formula in problem.predicate_defs_mut() {
        atomic_formula_skeleton::flatten(atomic_formula, map)?;
    }

    for atomic_function in problem.function_defs_mut() {
        atomic_function_skeleton::flatten(atomic_function, map)?;
    }

    // Update Domain and Problem Constraints
    expr::flatten(problem.domain_constraints_mut(), map)?;
    expr::flatten(problem.problem_constraints_mut(), map)?;

    // Update HTN Task Skeletons
    for task in problem.task_defs_mut() {
        task::flatten(task, map)?;
    }

    // Update Derived Predicates
    for derived_predicate in problem.derived_predicate_defs_mut() {
        derived_predicate::flatten(derived_predicate, map)?;
    }

    // Update Action signatures and effects
    for action in problem.action_defs_mut() {
        action::flatten(action, map)?;
    }

    // Update HTN Methods
    for method in problem.method_def_mut() {
        method::flatten(method, map)?;
    }

    // Update Goal conditions
    expr::flatten(problem.goal_mut(), map)?;

    // Update Initial Task Network (HTN)
    initial_task_network::flatten(problem.initial_task_network_mut(), map)?;

    Ok(())
}

/// Flattens all union (either) types in a lifted problem into primitive types pointing to unique pivots.
///
/// This function ensures that any "either" type is replaced by a "primitive" type
/// pointing to a canonical "pivot" type that contains only root types.
/// It performs a full resolution of the type hierarchy (flattening) and merges
/// equivalent types into single pivot definitions (canonicalization).
///
/// # Process
/// 1. Identifies all union types in the problem.
/// 2. Resolves each type down to its terminal root parents using an optimized DFS.
/// 3. Checks if a pivot with the same root parents already exists:
///    - If yes: Reuses the existing pivot identifier.
///    - If no: Generates a stable canonical name, interns it, and creates a new "either" pivot.
/// 4. Replaces the original union type with a `primitive` pointer to the pivot.
///
/// # Optimization
/// This implementation minimizes heap allocations by:
/// - Using reusable buffers (`stack_buffer`, `set_buffer`) for graph traversal.
/// - Utilizing the `Entry` API to avoid double hashing during canonicalization.
/// - Implementing "lazy naming" to only generate strings for newly created pivots.
///
/// # Parameters
/// - `problem`: The mutable lifted problem containing types to be flattened.
///
/// # Returns
/// A `Result` containing a `HashMap<Type<TypeID>, TypeID>` mapping each original
/// union structure to its corresponding pivot identifier. This mapping is essential
/// for updating predicates, actions, and objects during grounding.
///
/// # Errors
/// Returns a `LirError` if:
/// - A `TypeID` cannot be resolved in the problem's type table.
/// - A name cannot be resolved through the interner.
///
/// # Example
/// ```rust
/// let mut problem = LiftedProblem::new(...);
/// // After defining either types like 'either(a, b)'...
/// let mapping = flatten_types_def(&mut problem)?;
///
/// // All 'either' types in 'problem' are now 'primitive' pointers to flat pivots.
/// ```
fn flatten_types_def(
    problem: &mut LiftedProblem,
) -> Result<HashMap<Type<TypeID>, TypeID>, LirError> {
    // 1. Initialization
    let mut to_process = either_types(problem);
    let mut either_to_primitive: HashMap<Type<TypeID>, TypeID> = HashMap::new();
    let mut to_update: HashMap<TypeID, TypeID> = HashMap::new();

    // Map to merge types sharing identical root parents (Canonicalization)
    let mut parents_to_pivot: HashMap<Vec<TypeID>, TypeID> = HashMap::new();

    // Reusable buffers for DFS traversal to avoid repeated heap allocations
    let mut stack_buffer = Vec::with_capacity(32);
    let mut set_buffer = HashSet::with_capacity(32);

    // 2. Calculation Phase (Analysis + Pivot Creation)
    while let Some(original_id) = to_process.pop_front() {
        let ty_structure = problem.try_get_type(original_id)?.ty().clone();

        // Check if this exact structure was already mapped to a pivot
        if let Some(&pivot_id) = either_to_primitive.get(&ty_structure) {
            to_update.insert(original_id, pivot_id);
            continue;
        }

        // Resolve all terminal parents (roots) using the optimized DFS
        let flattened_parents_ty = get_parents(
            &ty_structure,
            problem,
            &mut stack_buffer,
            &mut set_buffer
        )?;

        // We use the raw vector of members as a key for canonicalization
        let parents_vec = flattened_parents_ty.members().to_vec();

        // Entry API: find existing pivot or create a new one efficiently
        let pivot_id = match parents_to_pivot.entry(parents_vec) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                // We only generate the name and intern it if we are actually creating a new pivot
                let new_name = make_either_type_name(problem, &flattened_parents_ty)?;
                let name_id = problem.interner_mut().intern_ident(new_name);

                let symbol_id = problem.add_type_symbol(name_id);
                let new_type_id = problem.add_type_defs(TypedSymbol::new(symbol_id, flattened_parents_ty))?;

                entry.insert(new_type_id);
                new_type_id
            }
        };

        either_to_primitive.insert(ty_structure, pivot_id);
        to_update.insert(original_id, pivot_id);
    }

    // 3. Application Phase (Update the problem definitions)
    for (old_id, new_pivot_id) in to_update {
        let new_ty_def = Type::primitive(new_pivot_id);
        if let Some(ts_mut) = problem.type_defs_mut().get_mut(old_id.as_usize()) {
            ts_mut.set_ty(new_ty_def);
        }
    }

    Ok(either_to_primitive)
}


/// Returns the identifiers of all either types in the problem.
///
/// # Parameters
/// - problem: the LiftedProblem containing all types.
///
/// # Returns
/// A VecDeque of Idents representing types that are unions (either types).
fn either_types(problem: &LiftedProblem) -> VecDeque<TypeID> {
    // We don't pre-allocate the full length because either types are usually
    // a small subset of the total types. VecDeque will grow as needed.
    problem
        .type_defs()
        .iter()
        .filter(|ts| ts.ty().is_either())
        .map(|ts| ts.symbol())
        .collect()
}

/// Returns all flattened root parent identifiers for a given `Type`, sorted.
///
/// This function performs a Depth-First Search (DFS) to resolve all nested types
/// (both `either` and `primitive` pointers) until only root types (empty definitions)
/// remain. It uses reusable buffers to minimize heap allocations.
///
/// # Parameters
/// - `types`: The `Type` to types.
/// - `problem`: The `LiftedProblem` context used to resolve `TypeID` definitions.
/// - `stack`: A reusable `Vec` buffer used for the traversal stack.
/// - `parent_set`: A reusable `HashSet` buffer used for duplicate removal.
///
/// # Returns
/// - `Ok(Type::new())` if no parents are found.
/// - `Ok(Type::primitive(id))` if exactly one root is found.
/// - `Ok(Type::either(ids))` if multiple roots are found (sorted).
/// - `Err(LirError)` if a `TypeID` cannot be resolved in the problem.
fn get_parents(
    ty: &Type<TypeID>,
    problem: &LiftedProblem,
    stack: &mut Vec<TypeID>,      // Buffer for the DFS traversal
    parent_set: &mut HashSet<TypeID>, // Buffer for duplicate removal
) -> Result<Type<TypeID>, LirError> {
    // 1. Reset buffers without deallocating their capacity
    stack.clear();
    parent_set.clear();

    // 2. Initialize stack from type members
    stack.extend(ty.iter());

    // 3. Flattening via DFS
    while let Some(m) = stack.pop() {
        let member_def = problem.try_get_type(m)?.ty();

        if member_def.is_empty() {
            // CAS 1: Root type found (no further parents)
            parent_set.insert(m);
        } else {
            // CAS 2: Complex type (either or primitive pivot).
            // We quantifiers its members to continue resolution.
            // Using rev() preserves the original order in the DFS stack.
            stack.extend(member_def.members().iter().rev());
        }
    }

    // 4. Final collection and sorting
    let mut parent_idents: Vec<TypeID> = parent_set.drain().collect();

    // Unstable sort is faster and sufficient for Copy types like TypeID
    parent_idents.sort_unstable();

    // 5. Canonical return based on the number of unique roots
    match parent_idents.len() {
        0 => Ok(Type::new()),
        1 => Ok(Type::primitive(parent_idents[0])),
        _ => Ok(Type::either(parent_idents)),
    }
}

/// Generates a canonical, stable name for an "either" type using efficient string slicing.
///
/// This function constructs a unique string representation for a union type (`either`) by
/// concatenating its member type names. To ensure the same set of types always produces
/// the same name regardless of order, the member names are sorted alphabetically.
///
/// # Optimization
/// This version minimizes heap allocations by:
/// 1. Working with references (`&str`) directly from the interner instead of cloning strings.
/// 2. Pre-calculating the exact total capacity required for the final `String`.
/// 3. Performing a single allocation for the result.
///
/// # Parameters
/// - `problem`: Reference to the `LiftedProblem` containing the type interner and symbol table.
/// - `types`: The `Type` (union/either) whose member identifiers will be used for the name.
///
/// # Returns
/// - `Ok(String)` representing the canonical name, e.g., `"either_parent1_parent2"`.
/// - `Err(LirError)` if any member identifier or string resolution fails.
fn make_either_type_name(problem: &LiftedProblem, ty: &Type<TypeID>) -> Result<String, LirError> {
    // 1. Pre-allocate a vector for references, not owned Strings
    let mut parent_names: Vec<&str> = Vec::with_capacity(ty.len());

    // 2. Resolve all identifiers to their string slices
    for id in ty.iter() {
        let string_id = problem.type_symbols().try_get_ident(*id)?;
        let name = problem.interner().try_resolve_ident(*string_id)?;
        parent_names.push(name);
    }

    // 3. Sort references (very fast, just compares pointers/slices)
    parent_names.sort_unstable();

    // 4. Calculate total capacity needed to avoid reallocations during formatting
    // Prefix + Sep + Names + Separators
    let total_len = EITHER_PREFIX.len()
        + EITHER_SEP.len()
        + parent_names.iter().map(|n| n.len()).sum::<usize>()
        + (parent_names.len().saturating_sub(1) * EITHER_SEP.len());

    let mut result = String::with_capacity(total_len);

    // 5. Build the final string
    result.push_str(EITHER_PREFIX);
    result.push_str(EITHER_SEP);

    for (i, name) in parent_names.iter().enumerate() {
        if i > 0 {
            result.push_str(EITHER_SEP);
        }
        result.push_str(name);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_types_def_simple() -> Result<(), LirError> {
        use crate::aiplan4rust::interner::StringInterner;
        use crate::aiplan4rust::lang::{Type, TypedSymbol, TypeID};
        use crate::aiplan4rust::lir::problem::LiftedProblem;
        use std::collections::HashSet;

        let mut interner = StringInterner::new();

        // 1. Prepare Identifiers
        let name_a = interner.intern_ident("a");
        let name_b = interner.intern_ident("b");
        let name_c = interner.intern_ident("c");
        let name_d = interner.intern_ident("d");

        // 2. Initialize the problem
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 3. Symbol Reservation phase
        // add_type_symbol allocates slots in the vector and returns the TypeID
        let id_a = problem.add_type_symbol(name_a);
        let id_b = problem.add_type_symbol(name_b);
        let id_c = problem.add_type_symbol(name_c);
        let id_d = problem.add_type_symbol(name_d);

        // 4. Data Injection phase
        // We define a, b, c as primitives of 'object' (roots for the purpose of this test)
        // Note: In a real LIR problem, roots often have an empty definition Type::new()
        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_c, Type::new()))?;

        // Define 'd' as a union (either) of a, b, and c
        let either_abc_type = Type::either(vec![id_a, id_b, id_c]);
        problem.add_type_defs(TypedSymbol::new(id_d, either_abc_type))?;

        // --- BEFORE Flattening Output ---
        println!("\nTypes before flattening:");
        for (idx, ts) in problem.type_defs().iter().enumerate() {
            let type_id = TypeID::from(idx);
            let string_id = problem.type_symbols().get_ident(type_id)
                .expect("TypeID must have an associated name");

            let name = problem.interner().try_resolve_ident(*string_id)?;
            println!("{}: {:?}", name, ts.ty().members());
        }

        // 5. Execute flattening
        super::flatten_types_def(&mut problem).expect("Flattening failed");

        // --- AFTER Flattening Output ---
        println!("\nTypes after flattening:");
        for (idx, ts) in problem.type_defs().iter().enumerate() {
            let type_id = TypeID::from(idx);
            let string_id = problem.type_symbols().try_get_ident(type_id)?;
            let name = problem.interner().try_resolve_ident(*string_id)?;
            println!("{}: {:?}", name, ts.ty().members());
        }

        // 6. Final Verifications

        // Type 'd' must now be a primitive pointing to the new pivot type 'either_a_b_c'
        let sym_d = problem.try_get_type(id_d).expect("Type 'd' must exist");
        assert!(sym_d.ty().is_primitive(), "Type 'd' should have become primitive");
        assert_eq!(sym_d.ty().members().len(), 1, "Type 'd' should point to exactly one pivot");

        // Retrieve the Pivot Type created during flattening
        let pivot_type_id = sym_d.ty().members()[0];
        let pivot_symbol = problem.try_get_type(pivot_type_id)?;

        // Verify Pivot internal structure
        assert!(pivot_symbol.ty().is_either(), "The pivot itself must be an 'either' type");
        let pivot_members = pivot_symbol.ty().members();
        assert_eq!(pivot_members.len(), 3, "Pivot should contain exactly 3 roots (a, b, c)");
        assert!(pivot_members.contains(&id_a));
        assert!(pivot_members.contains(&id_b));
        assert!(pivot_members.contains(&id_c));

        // Verify Pivot naming in the interner
        let pivot_string_id = problem.type_symbols().get_ident(pivot_type_id)
            .expect("Pivot type must be registered in the symbol table");

        let pivot_name = problem.interner().try_resolve_ident(*pivot_string_id)?;
        println!("New pivot created: {}", pivot_name);

        assert!(pivot_name.starts_with("either_"), "Pivot name should start with 'either_'");
        // Depending on your make_either_type_name logic, it might contain names or IDs
        assert!(pivot_name.contains("a") || pivot_name.contains("b") || pivot_name.contains("c"),
                "Pivot name should be descriptive of its members");

        Ok(())
    }

    #[test]
    fn test_flatten_types_def_complex() -> Result<(), LirError> {
        use crate::aiplan4rust::interner::StringInterner;
        use crate::aiplan4rust::lang::{Type, TypedSymbol};
        use crate::aiplan4rust::lir::problem::LiftedProblem;
        use std::collections::HashSet;

        let mut interner = StringInterner::new();

        // 1. Prepare Identifiers
        let name_a = interner.intern_ident("a");
        let name_b = interner.intern_ident("b");
        let name_c = interner.intern_ident("c");
        let name_d = interner.intern_ident("d");
        let name_e = interner.intern_ident("e");
        let name_f = interner.intern_ident("f");

        // 2. Initialize the problem
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 3. Reservation phase (add_type_symbol)
        let id_a = problem.add_type_symbol(name_a);
        let id_b = problem.add_type_symbol(name_b);
        let id_c = problem.add_type_symbol(name_c);
        let id_d = problem.add_type_symbol(name_d);
        let id_e = problem.add_type_symbol(name_e);
        let id_f = problem.add_type_symbol(name_f);

        // 4. Injection phase (add_type)
        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?; // Root type
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?; // Root type

        // Define complex hierarchy:
        // c = either(a, b)
        problem.add_type_defs(TypedSymbol::new(id_c, Type::either(vec![id_a, id_b])))?;
        // d = either(a, c) -> should resolve to {a, b}
        problem.add_type_defs(TypedSymbol::new(id_d, Type::either(vec![id_a, id_c])))?;
        // e = either(c, d) -> should resolve to {a, b}
        problem.add_type_defs(TypedSymbol::new(id_e, Type::either(vec![id_c, id_d])))?;
        // f = either(b, c) -> should resolve to {a, b}
        problem.add_type_defs(TypedSymbol::new(id_f, Type::either(vec![id_b, id_c])))?;

        // 5. Execute flattening
        super::flatten_types_def(&mut problem)?;

        // 6. VERIFICATIONS

        // We expect c, d, e, and f to all point to the EXACT SAME pivot ID
        let mut pivot_ids = HashSet::new();

        for &id in &[id_c, id_d, id_e, id_f] {
            let ts = problem.try_get_type(id)?;

            // A. Verify type is now a Primitive (Single pointer)
            let type_name = problem.interner().try_resolve_ident(
                *problem.type_symbols().try_get_ident(id)?
            )?;

            assert!(
                ts.ty().is_primitive(),
                "Type '{}' should be a primitive after flattening", type_name
            );

            // B. Collect the pivot ID
            let pivot_id = ts.ty().members()[0];
            pivot_ids.insert(pivot_id);

            // C. Verify the pivot definition
            let pivot_ts = problem.try_get_type(pivot_id)?;
            assert!(
                pivot_ts.ty().is_either(),
                "Pivot for '{}' must be an 'either' type (the actual definition)", type_name
            );

            // D. Verify the pivot contains ONLY atomic roots (Full Flattening)
            let members = pivot_ts.ty().members();
            assert_eq!(
                members.len(), 2,
                "Pivot for '{}' should have exactly 2 members (a and b)", type_name
            );
            assert!(members.contains(&id_a), "Pivot for '{}' is missing root 'a'", type_name);
            assert!(members.contains(&id_b), "Pivot for '{}' is missing root 'b'", type_name);

            // E. Verify no residual recursion (pivot members must be empty/roots)
            for &m_id in members {
                assert!(
                    problem.try_get_type(m_id)?.ty().is_empty(),
                    "Pivot member must be a root type, not another pointer"
                );
            }
        }

        // F. CANONICALIZATION CHECK (Deduplication)
        assert_eq!(
            pivot_ids.len(),
            1,
            "Redundant types (c, d, e, f) should all point to the same unique pivot"
        );

        // G. Verify pivot naming convention
        let final_pivot_id = *pivot_ids.iter().next().unwrap();
        let pivot_string_id = problem.type_symbols().try_get_ident(final_pivot_id)?;
        let pivot_name = problem.interner().try_resolve_ident(*pivot_string_id)?;

        assert!(
            pivot_name.starts_with("either_"),
            "Pivot name '{}' does not follow the naming convention", pivot_name
        );

        Ok(())
    }

    #[test]
    fn test_flatten_diamond_dependency() -> Result<(), LirError> {
        use crate::aiplan4rust::interner::StringInterner;
        use crate::aiplan4rust::lang::{Type, TypedSymbol};
        use crate::aiplan4rust::lir::problem::LiftedProblem;
        use std::collections::HashSet;

        let interner = StringInterner::new();
        let mut problem = LiftedProblem::new(interner, HashSet::new());

        // 1. Create roots
        let a = problem.interner_mut().intern_ident("a");
        let id_a = problem.add_type_symbol(a);
        let b = problem.interner_mut().intern_ident("b");
        let id_b = problem.add_type_symbol(b);
        let e = problem.interner_mut().intern_ident("e");
        let id_e = problem.add_type_symbol(e);

        problem.add_type_defs(TypedSymbol::new(id_a, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_b, Type::new()))?;
        problem.add_type_defs(TypedSymbol::new(id_e, Type::new()))?;

        // 2. Intermediate types
        let c = problem.interner_mut().intern_ident("c");
        let id_c = problem.add_type_symbol(c);
        let d = problem.interner_mut().intern_ident("d");
        let id_d = problem.add_type_symbol(d);
        problem.add_type_defs(TypedSymbol::new(id_c, Type::either(vec![id_a, id_b])))?;
        problem.add_type_defs(TypedSymbol::new(id_d, Type::either(vec![id_b, id_e])))?;

        // 3. Diamond types: both resolve to {a, b, e}
        let f = problem.interner_mut().intern_ident("f");
        let id_f = problem.add_type_symbol(f);
        let g = problem.interner_mut().intern_ident("g");
        let id_g = problem.add_type_symbol(g);
        problem.add_type_defs(TypedSymbol::new(id_f, Type::either(vec![id_c, id_d])))?; // {a, b} + {b, e}
        problem.add_type_defs(TypedSymbol::new(id_g, Type::either(vec![id_a, id_d])))?; // {a} + {b, e}

        // 4. Flatten
        super::flatten_types_def(&mut problem)?;

        // 5. Verification
        let pivot_f = problem.try_get_type(id_f)?.ty().members()[0];
        let pivot_g = problem.try_get_type(id_g)?.ty().members()[0];

        // Check canonicalization (the most important part)
        assert_eq!(pivot_f, pivot_g, "F and G should point to the exact same pivot because they have identical roots");

        let final_pivot = problem.try_get_type(pivot_f)?.ty();
        assert_eq!(final_pivot.members().len(), 3, "The pivot should have exactly 3 unique roots: a, b, e");
        assert!(final_pivot.members().contains(&id_a));
        assert!(final_pivot.members().contains(&id_b));
        assert!(final_pivot.members().contains(&id_e));

        Ok(())
    }
}
