/*use crate::aiplan4rust::lir::problem::LiftedProblem;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::aiplan4rust::interner::{Ident, InternerError};
use crate::aiplan4rust::lang::{Type, TypedSymbol};


const EITHER_PREFIX: &str = "either";
const EITHER_SEP: &str = "_";


/// Flatten all `either` types in a LiftedProblem.
///
/// This function will:
/// - Replace `either` types with new primitive types (one per combination)
/// - Update all type references in objects and functions accordingly
/// - Ensure that after flattening, all types have exactly one member
pub fn flatten_either_types(problem: &mut LiftedProblem) -> Result<(), InternerError> {

    flatten_types_def(problem)?;

    Ok(())

}

/// Fixed-point flattening for all nested `either` types.
///
/// Iterates over all `either` types in `problem.types()`, creates the corresponding
/// flattened types, and updates any affected child types until all `either` types
/// have been processed.
///
/// # Parameters
/// - `problem`: Contains all `TypedSymbol`s and the interner.
/// - `child_to_parents`: Map from a type `Ident` to its parent `TypedSymbol`s.
///
/// # Returns
/// `InternerError` if any interning operation fails.
fn flatten_types_def(
    problem: &mut LiftedProblem,
) -> Result<(), InternerError> {
    // Build a quick lookup from a type Ident to its TypedSymbol for fast access
    let symbol_lookup = build_symbol_lookup(problem);

    // Queue of indices of either types to process
    let mut to_process: VecDeque<usize> = either_type_indices(problem).into();

    // Cache to avoid recalculating an either type that has already been flattened
    let mut cache: HashMap<Type, Ident> = HashMap::new();

    while let Some(idx) = to_process.pop_front() {
        let symbol = &mut problem.types()[idx];

        if let Some(&ident) = cache.get(symbol.ty()) {
            // Type already flattened, just update the TypedSymbol
            symbol.ty_mut().set_members(vec![ident]);
            continue;
        }

        // Flatten the either type and get the index of the new type
        let new_index = flatten_type(symbol, problem, &symbol_lookup)?;

        // Enqueue any newly created either type for processing
        to_process.push_back(new_index);
    }

    Ok(())
}

/// Builds a fast lookup table from a type `Ident` to its `TypedSymbol`.
///
/// This allows efficient access to a `TypedSymbol` by its identifier
/// without iterating over the full list of types in the problem.
fn build_symbol_lookup(problem: &Lifpour etedProblem) -> HashMap<Ident, &TypedSymbol> {
    let mut map: HashMap<Ident, &TypedSymbol> = HashMap::new();

    for sym in problem.types() {
        map.insert(sym.symbol(), sym);
    }

    map
}

/// Returns the indices of all `either` types in the problem.
///
/// This function iterates through all `TypedSymbol`s in the given `LiftedProblem`
/// and collects the indices of those whose type is an `either`.
///
/// # Parameters
/// - `problem`: The `LiftedProblem` containing all type definitions.
///
/// # Returns
/// A `Vec<usize>` containing the indices in `problem.types()` of all `either` types.
fn either_type_indices(problem: &LiftedProblem) -> Vec<usize> {
    let mut indices = Vec::new();

    for (i, sym) in problem.types().iter().enumerate() {
        if sym.ty().is_either() {
            indices.push(i);
        }
    }

    indices
}

/// Creates a flattened `either` type for a given `TypedSymbol`.
///
/// This function generates a new `TypedSymbol` that represents the flattened
/// union of the symbol's member types. It updates the original symbol to refer
/// to the new flattened type. This version does **not use a cache**; each call
/// will create a new flattened type if needed.
///
/// # Parameters
/// - `symbol`: The `TypedSymbol` representing an `either` type to flatten.
/// - `problem`: The `LiftedProblem` containing all type definitions and the interner.
/// - `symbol_lookup`: A map from a type `Ident` to its parent `TypedSymbol`s.
///
/// # Returns
/// Returns the **index** of the newly added `TypedSymbol` in `problem.types()`.
///
/// # Errors
/// Returns `InternerError` if interning a type name fails or resolving a parent identifier fails.
pub fn flatten_type(
    symbol: &mut TypedSymbol,
    problem: &mut LiftedProblem,
    symbol_lookup: &HashMap<Ident, Vec<&TypedSymbol>>,
) -> Result<usize, InternerError> {
    let ty = symbol.ty();

    // Ensure this is an either type
    debug_assert!(ty.len() > 1, "flatten_type called on non-either type");

    // Get flattened parents using helper
    let parents = get_parents(ty, symbol_lookup);

    // Generate a stable name for the either type
    let type_name = make_either_type_name(problem, &parents)?;

    // Intern the new type identifier
    let type_ident = problem.interner_mut().intern_ident(type_name)?;

    // Create the new flattened TypedSymbol
    let new_parents = Type::either(parents);
    let new_symbol = TypedSymbol::new(type_ident, new_parents);
    problem.add_type(new_symbol);

    // Update the original symbol to point to the new flattened type
    symbol.ty_mut().set_members(vec![type_ident]);

    // Return the index of the newly added type
    Ok(problem.types().len() - 1)
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
fn get_parents(
    ty: &Type,
    symbol_lookup: &HashMap<Ident, Vec<&TypedSymbol>>,
) -> Vec<Ident> {
    let mut parent_set = HashSet::new();

    for &m in &ty.iter() {
        let parents = symbol_lookup.get(&m).unwrap(); // unwrap intentional
        parent_set.extend(parents.member()); // collect members of parents
    }

    let mut parent_idents: Vec<Ident> = parent_set.into_iter().collect();
    parent_idents.sort();
    parent_idents
}

/// Generates a stable name for an "either" type based on its parent type identifiers.
///
/// # Parameters
/// - `problem`: Reference to the `LiftedProblem` containing the type interner.
/// - `parents`: Slice of `Ident` representing the parent types of the either type.
///
/// # Returns
/// Returns a `String` representing the canonical name of the either type, e.g., `"either_parent1_parent2"`.
/// Returns an `InternerError` if any parent identifier cannot be resolved.
fn make_either_type_name(
    problem: &LiftedProblem,
    parents: &[Ident],
) -> Result<String, InternerError> {
    let mut parent_names = Vec::with_capacity(parents.len());

    for &id in parents {
        let name = problem.interner().try_resolve_ident(id)?;
        parent_names.push(name);
    }

    Ok(format!("{}{}{}", EITHER_PREFIX, EITHER_SEP, parent_names.join(EITHER_SEP)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::interner::StringInterner;
    use crate::aiplan4rust::lang::{Type, TypedSymbol};
    use crate::aiplan4rust::lir::problem::LiftedProblem;

    #[test]
    fn test_flatten_simple_either() {
        let mut interner = StringInterner::new();

        // Crée des identifiants pour les types de base
        let a = interner.intern_ident("a");
        let b = interner.intern_ident("b");
        let c = interner.intern_ident("c");

        // Types primitifs (pas de nesting)
        let type_a = Type::primitive(a);
        let type_b = Type::primitive(b);
        let type_c = Type::primitive(c);

        // TypedSymbols
        let sym_a = TypedSymbol::new(a, type_a);
        let sym_b = TypedSymbol::new(b, type_b);
        let sym_c = TypedSymbol::new(c, type_c);

        // Either type: either A B C
        let either_abc_type = Type::either(vec![a, b, c]);
        let d = interner.intern_ident("d");
        let sym_d = TypedSymbol::new(d, either_abc_type);

        // Construire le LiftedProblem
        let mut problem = LiftedProblem::new(interner, HashSet::new());
        problem.add_type(sym_a);
        problem.add_type(sym_b);
        problem.add_type(sym_c);
        problem.add_type(sym_d);

        // Appeler flatten_types_def
        flatten_types_def(&mut problem).unwrap();

        // Vérifie que le either type a été aplati
        let flattened_sym = &problem.types()[3];
        assert_eq!(flattened_sym.ty().members().len(), 1);

        // Vérifie que le nom contient "either"
        let ident_name = problem.interner().try_resolve_ident(flattened_sym.ty().members()[0]).unwrap();
        assert!(ident_name.starts_with("either_"));

        // Vérifie que tous les membres de l'either sont inclus dans le nom
        assert!(ident_name.contains("a"));
        assert!(ident_name.contains("b"));
        assert!(ident_name.contains("c"));
    }
}
*/
