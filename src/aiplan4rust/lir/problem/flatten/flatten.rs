#![allow(dead_code)]

use crate::aiplan4rust::interner::{Ident, InternerError};
use crate::aiplan4rust::lang::{RemapTypes, Type, TypedSymbol};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::aiplan4rust::lir::LirError;

const EITHER_PREFIX: &str = "either";
const EITHER_SEP: &str = "_";

/// Flattens all union types (`Type::Either`) in a `LiftedProblem`.
///
/// # Parameters
/// - `problem`: The `LiftedProblem` to be flattened in place.
///
/// # Returns
/// - `Ok(())` if all types were successfully flattened.
/// - `Err(LirError)` if any type cannot be flattened due to a missing mapping.
pub fn flatten_types(problem: &mut LiftedProblem) -> Result<(), LirError> {
    let flatten_types_map = flatten_types_def(problem)?;

    for constant in problem.constants_mut() {
        constant.remap_types(&flatten_types_map)?;
    }

    for predicate in problem.predicates_mut() {
        predicate.remap_types(&flatten_types_map)?;
    }

    for function in problem.functions_mut() {
        function.remap_types(&flatten_types_map)?;
    }

    problem.domain_constraints_mut().remap_types(&flatten_types_map)?;

    for task in problem.tasks_mut() {
        task.remap_types(&flatten_types_map)?;
    }

    for action in problem.actions_mut() {
        action.remap_types(&flatten_types_map)?;
    }

    for methods in problem.methods_mut() {
        methods.remap_types(&flatten_types_map)?;
    }

    for object in problem.objects_mut() {
        object.remap_types(&flatten_types_map)?;
    }

    problem.goal_mut().remap_types(&flatten_types_map)?;

    problem.problem_constraints_mut().remap_types(&flatten_types_map)?;

    problem.initial_task_network_mut().remap_types(&flatten_types_map)?;

    Ok(())
}


/// Flattens all union (either) types in a lifted problem into primitive types.
///
/// For each `TypedSymbol` in the problem that has an "either" type:
/// 1. Computes a unique flattened type name based on its parents.
/// 2. Interns a new identifier for this flattened type.
/// 3. Updates the original `TypedSymbol` to reference the new primitive type.
/// 4. Records the mapping from the original union type to the new primitive identifier.
///
/// # Parameters
/// - `problem`: The mutable lifted problem containing types, constants, etc.
///
/// # Returns
/// A `HashMap<Type, Ident>` mapping each original union (`either`) type to the
/// corresponding new primitive type identifier. This can be used to update predicates,
/// actions, constants, etc., to refer to the flattened types.
///
/// # Errors
/// Returns a `GroundingError` if any type cannot be found or if parent resolution fails.
///
/// # Notes
/// - The function also maintains an internal cache to avoid flattening the same type multiple times.
/// - After flattening, all original union types in the problem are replaced by primitive types,
///   while new `TypedSymbol`s representing the union types are added to the problem for reference.
///
/// # Example
/// ```rust
/// let mut problem = LiftedProblem::new();
/// let mapping = flatten_types_def(&mut problem)?;
/// for (union_type, prim_ident) in mapping {
///     println!("Union {:?} -> Primitive {:?}", union_type, prim_ident);
/// }
/// ```
fn flatten_types_def(
    problem: &mut LiftedProblem,
) -> Result<HashMap<Type, Ident>, LirError> {
    // Queue of either types to process, represented by their Ident
    let mut to_process = either_types(problem);

    // Maps original type identifiers to their flattened primitive Type.
    // We'll use this later to update the types in the problem.
    let mut to_update: HashMap<Ident, Type> = HashMap::with_capacity(problem.types().count());

    // Maps union (either) types to the new primitive Ident representing them.
    // This is returned so we can replace union types in constants, predicates, etc.
    let mut either_to_primitive = HashMap::with_capacity(problem.types().count());

    // Cache to avoid recalculating a type that has already been flattened
    let mut cache: HashMap<Ident, Ident> = HashMap::with_capacity(problem.types().count());

    while let Some(ident) = to_process.pop_front() {
        // Get a reference to the TypedSymbol
        let ts = problem.try_get_type(ident)?;
        let ty_ident = ts.symbol(); // unique identifier of this type

        // If already flattened, just record the mapping
        if let Some(&cached_ident) = cache.get(&ty_ident) {
            to_update.insert(ident, Type::primitive(cached_ident));
            continue;
        }

        // Clone the type to break the borrow and compute parents
        let ty = ts.ty().clone();
        let parents = get_parents(&ty, problem)?;
        let type_name = make_either_type_name(problem, &ty)?;

        // Intern a new type identifier
        let type_ident = problem.interner_mut().intern_ident(type_name);

        // Map the original union type to the new primitive identifier
        either_to_primitive.insert(ty, type_ident);

        // Update the original TypedSymbol to point to the new flattened type
        to_update.insert(ident, Type::primitive(type_ident));

        // Create a new TypedSymbol representing the union type itself
        let new_symbol = TypedSymbol::new(type_ident, Type::either(parents));
        if new_symbol.ty().is_either() {
            to_process.push_back(type_ident);
        }
        problem.add_type(new_symbol);

        // Update the cache
        cache.insert(ty_ident, type_ident);
    }

    // Replace all original types with their flattened primitive types
    for (ident, new_type) in to_update {
        let ts_mut = problem.try_get_type_mut(ident)?;
        ts_mut.set_ty(new_type);
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
fn either_types(problem: &LiftedProblem) -> VecDeque<Ident> {
    let mut queue = VecDeque::with_capacity(problem.types().count());
    for ts in problem.types() {
        if ts.ty().is_either() {
            queue.push_back(ts.symbol());
        }
    }
    queue
}

/// Returns all flattened parent identifiers for a given `Type`, sorted.
///
/// # Parameters
/// - `ty`: The `Type` whose parents are to be collected. Assumes `ty` may be an either type.
/// - `symbol_lookup`: A map from a type `Ident` to its direct parent `TypedSymbol`s.
///
/// # Returns
/// A sorted `Vec<Ident>` containing all parent identifiers of the type, flattened.
///
/// # Panics
/// Panics if any member of `ty` is not found in `symbol_lookup`.
fn get_parents(ty: &Type, problem: &LiftedProblem) -> Result<Vec<Ident>, LirError> {
    let mut parent_set = HashSet::new();

    for &m in ty.iter() {
        let parent_type = problem.try_get_type(m)?.ty();
        parent_set.extend(parent_type.members()); // collect members of parents
    }

    let mut parent_idents: Vec<Ident> = parent_set.into_iter().collect();
    parent_idents.sort();
    Ok(parent_idents)
}

/// Generates a canonical name for an "either" type based on its member type identifiers.
///
/// This function constructs a stable string name for a union type (`either`) by
/// concatenating the string representations of all its member types, separated
/// by `EITHER_SEP` and prefixed with `EITHER_PREFIX`.
///
/// # Parameters
/// - `problem`: Reference to the `LiftedProblem` containing the type interner.
/// - `ty`: The `Type` whose member identifiers will be used to generate the name.
///
/// # Returns
/// - A `String` representing the canonical name of the either type,
///   e.g., `"either_parent1_parent2"`.
/// - Returns an `InternerError` if any member identifier cannot be resolved.
fn make_either_type_name(problem: &LiftedProblem, ty: &Type) -> Result<String, InternerError> {
    let mut parent_names = Vec::with_capacity(ty.len());

    // Boucle explicite pour récupérer les noms des membres
    for id in ty.iter() {
        let name = problem.interner().try_resolve_ident(*id)?;
        parent_names.push(name.to_string());
    }

    // Trier pour obtenir un nom stable
    parent_names.sort_unstable();

    // Concaténer avec le préfixe
    let result = format!(
        "{}{}{}",
        EITHER_PREFIX,
        EITHER_SEP,
        parent_names.join(EITHER_SEP)
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lang::{Type, TypedSymbol};
    use crate::aiplan4rust::lir::problem::LiftedProblem;

    #[test]
    fn test_flatten_types_def_simple() {
        let mut interner = StringInterner::new();

        // Crée des identifiants pour les types de base
        let a = interner.intern_ident("a");
        let b = interner.intern_ident("b");
        let c = interner.intern_ident("c");


        let sym_obj = TypedSymbol::new(StringInterner::IDENT_OBJECT, Type::new());

        // TypedSymbols
        let sym_a = TypedSymbol::new(a, Type::primitive(StringInterner::IDENT_OBJECT));
        let sym_b = TypedSymbol::new(b, Type::primitive(StringInterner::IDENT_OBJECT));
        let sym_c = TypedSymbol::new(c, Type::primitive(StringInterner::IDENT_OBJECT));

        // Either type
        let either_abc_type = Type::either(vec![a, b, c]);
        let d = interner.intern_ident("d");
        let sym_d = TypedSymbol::new(d, either_abc_type);

        // Construire le LiftedProblem
        let mut problem = LiftedProblem::new(interner, HashSet::new());
        problem.add_type(sym_obj);
        problem.add_type(sym_a);
        problem.add_type(sym_b);
        problem.add_type(sym_c);
        problem.add_type(sym_d);

        println!("\nTypes before flattening:");
        for ts in problem.types() {
            let members: Vec<String> = ts
                .ty()
                .members()
                .iter()
                .map(|id| {
                    problem
                        .interner()
                        .try_resolve_ident(*id)
                        .unwrap()
                        .to_string()
                })
                .collect();
            println!(
                "{}: {:?}",
                problem.interner().try_resolve_ident(ts.symbol()).unwrap(),
                members
            );
        }

        flatten_types_def(&mut problem).unwrap();

        println!("Types after flattening:");
        for ts in problem.types() {
            let members: Vec<String> = ts
                .ty()
                .members()
                .iter()
                .map(|id| {
                    problem
                        .interner()
                        .try_resolve_ident(*id)
                        .unwrap()
                        .to_string()
                })
                .collect();
            println!(
                "{}: {:?}",
                problem.interner().try_resolve_ident(ts.symbol()).unwrap(),
                members
            );
        }

        let flattened_sym = &problem.get_type(d).unwrap();
        assert_eq!(flattened_sym.ty().members().len(), 1);

        let ident_name = problem
            .interner()
            .try_resolve_ident(flattened_sym.ty().members()[0])
            .unwrap();
        assert!(ident_name.starts_with("either_"));

        assert!(ident_name.contains("a"));
        assert!(ident_name.contains("b"));
        assert!(ident_name.contains("c"));
    }

    #[test]
    fn test_flatten_types_def_complexe() {
        use crate::aiplan4rust::interner::StringInterner;
        use crate::aiplan4rust::lang::{Type, TypedSymbol};
        use crate::aiplan4rust::lir::problem::LiftedProblem;
        use std::collections::HashSet;

        let mut interner = StringInterner::new();

        // Create the basic ident
        let a = interner.intern_ident("a");
        let sym_a = TypedSymbol::new(a, Type::new());

        let b = interner.intern_ident("b");
        let sym_b = TypedSymbol::new(b, Type::new());

        // Either types complexes
        let c = interner.intern_ident("c");
        let sym_c = TypedSymbol::new(c, Type::either(vec![a, b]));

        let d = interner.intern_ident("d");
        let sym_d = TypedSymbol::new(d, Type::either(vec![a, c]));

        let e = interner.intern_ident("e");
        let sym_e = TypedSymbol::new(e, Type::either(vec![c, d]));

        let f = interner.intern_ident("f");
        let sym_f = TypedSymbol::new(f, Type::either(vec![b, c]));

        // Build theLiftedProblem
        let mut problem = LiftedProblem::new(interner, HashSet::new());
        for ts in [sym_a, sym_b, sym_c, sym_d, sym_e, sym_f] {
            problem.add_type(ts);
        }

        println!("\nTypes before flattening:");
        for ts in problem.types() {
            let members: Vec<String> = ts
                .ty()
                .members()
                .iter()
                .map(|id| {
                    problem
                        .interner()
                        .try_resolve_ident(*id)
                        .unwrap()
                        .to_string()
                })
                .collect();
            println!(
                "{}: {:?}",
                problem.interner().try_resolve_ident(ts.symbol()).unwrap(),
                members
            );
        }

        super::flatten_types_def(&mut problem).unwrap();

        println!("Types after flattening:");
        for ts in problem.types() {
            let members: Vec<String> = ts
                .ty()
                .members()
                .iter()
                .map(|id| {
                    problem
                        .interner()
                        .try_resolve_ident(*id)
                        .unwrap()
                        .to_string()
                })
                .collect();
            println!(
                "{}: {:?}",
                problem.interner().try_resolve_ident(ts.symbol()).unwrap(),
                members
            );
        }

        // Vérification simple : tous les either ont exactement un membre aplati
        for &either_ident in &[a, d, f] {
            let ts = problem.get_type(either_ident).unwrap();
            let num_members = ts.ty().members().len();
            assert!(
                num_members == 0 || num_members == 1,
                "Either type {:?} not flattened properly: expected 0 or 1 member, got {}",
                either_ident,
                num_members
            );
        }
    }
}
