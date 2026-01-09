use std::collections::{HashMap, HashSet};
use toml::value::Index;
use crate::aiplan4rust::grounding::problem::{Function, IndexTable, Type, ValueDomain};
use crate::aiplan4rust::grounding::problem::index_table::IndexTableError;
use crate::aiplan4rust::interner::Ident;
use crate::aiplan4rust::lang::{TypedList, TypedSymbol};
use crate::aiplan4rust::lir::atomic_skeleton::{AtomicFormulaSkeleton, AtomicFunctionSkeleton};
use crate::aiplan4rust::lir::problem::LiftedProblem;

/// Builds an `IndexTable` containing all type identifiers from the given typed symbols.
///
/// This includes:
/// - Each symbol's own identifier.
/// - All identifiers of the members of its associated type.
///
/// # Parameters
/// - `typed_symbols`: Map from `Ident` to `TypedSymbol` representing all types in the domain.
///
/// # Returns
/// An `IndexTable` containing all type identifiers referenced by the typed symbols.
pub fn build_type_symbols_table(
    problem : &LiftedProblem,
) -> IndexTable {
    let mut table = IndexTable::new();

    for ts in problem.types() {
        // Add the symbol's own identifier
        table.insert(ts.symbol());

        // Add all member identifiers of the type
        for ty_id in ts.ty().iter() {
            table.insert(*ty_id);
        }
    }

    table
}

pub fn build_types_table(
    typed_symbols: &HashSet<TypedSymbol>,
    typed_symbols_table: &IndexTable,
) -> Result<Vec<Type>, IndexTableError> {
    // On doit connaître le nombre total de symboles pour pré-allouer la table
    let mut types: Vec<Type> = vec![Type::default(); typed_symbols_table.len()];

    for ts in typed_symbols {
        // Index du symbole principal
        let symbol_idx = typed_symbols_table.try_index(&ts.symbol())?;

        // Construire la liste des supertypes pour ce symbole
        let mut super_types = Vec::with_capacity(ts.ty().len());
        for id in ts.ty().iter() {
            let idx = typed_symbols_table.try_index(id)?;
            super_types.push(idx);
        }

        // On affecte la liste des supertypes dans la table globale
        types[symbol_idx] = Type::either(super_types);
    }

    Ok(types)
}

/// Builds an index table for all predicates from a list of `AtomicFormulaSkeleton`.
///
/// Each unique predicate name is inserted into the `IndexTable` with a unique index,
/// allowing fast lookup of predicates by name or index.
///
/// # Parameters
/// - `predicates`: A slice of predicates from a lifted problem.
///
/// # Returns
/// An `IndexTable` mapping each predicate's identifier to a unique index.
///
/// # Example
/// ```rust
/// # use aiplan4rust::lang::{AtomicFormulaSkeleton, Ident};
/// # use aiplan4rust::grounding::problem::IndexTable;
/// let preds = vec![AtomicFormulaSkeleton::new(Ident::new("at"), vec![])];
/// let table = build_predicates_symbols_table(&preds);
/// assert!(table.contains(&Ident::new("at")));
/// ```
pub fn build_predicates_symbols_table(predicates: &[AtomicFormulaSkeleton]) -> IndexTable {
    let mut table = IndexTable::new();

    for pred in predicates {
        table.insert(pred.symbol());
    }

    table
}

/// Builds an index table for all numeric functions from a list of `AtomicFormulaSkeleton`.
///
/// Each unique function name is inserted into the `IndexTable` with a unique index,
/// allowing fast lookup of functions by name or index.
///
/// # Parameters
/// - `functions`: A slice of numeric functions from a lifted problem.
///
/// # Returns
/// An `IndexTable` mapping each function's identifier to a unique index.
///
/// # Example
/// ```rust
/// # use aiplan4rust::lang::{AtomicFormulaSkeleton, Ident};
/// # use aiplan4rust::grounding::problem::IndexTable;
/// let funcs = vec![AtomicFormulaSkeleton::new(Ident::new("distance"), vec![])];
/// let table = build_functions_symbols_table(&funcs);
/// assert!(table.contains(&Ident::new("distance")));
/// ```
pub fn build_functions_symbols_table(functions: &[AtomicFunctionSkeleton]) -> IndexTable {
    let mut table = IndexTable::new();

    for func in functions {
        let ret_type = func.return_type();
        if ret_type.is_number() {
            table.insert(func.symbol());
        }
    }

    table
}

/// Builds the symbol table for all objects in the problem.
///
/// Objects include:
/// - Constants,
/// - Declared objects,
/// - Functions with non-numeric return type.
///
/// # Parameters
/// - `constants`: Set of constant symbols.
/// - `objects`: Set of object symbols.
/// - `functions`: List of numeric and non-numeric functions (filtering only non-number).
///
/// # Returns
/// An `IndexTable` containing all object symbols.
pub fn build_objects_symbols_table(
    constants: &HashSet<TypedSymbol>,
    objects: &HashSet<TypedSymbol>,
    functions: &[AtomicFunctionSkeleton],
) -> IndexTable {
    let mut table = IndexTable::new();

    // Add constants
    for c in constants {
        table.insert(c.symbol());
    }

    // Add objects
    for obj in objects {
        table.insert(obj.symbol());
    }

    // Add objects fluents
    for func in functions {
        let ret_type = func.return_type();
        if !ret_type.is_number() {
            table.insert(func.symbol());
        }
    }

    table
}

/// Builds the list of object constants from lifted constants and objects.
///
/// Each object is represented as a `Function` with empty arguments,
/// and its return type is determined by the associated type(s).
///
/// # Parameters
/// - `constants`: Set of lifted constants (`TypedSymbol`).
/// - `objects`: Set of lifted objects (`TypedSymbol`).
/// - `type_symbols`: IndexTable mapping type identifiers to indices.
/// - `objects_symbols`: IndexTable mapping object identifiers to indices.
///
/// # Returns
/// A vector of `Function` representing all objects in the problem.
pub fn build_objects_table(
    constants: &HashSet<TypedSymbol>,
    objects: &HashSet<TypedSymbol>,
    type_symbols: &IndexTable,
    objects_symbols: &IndexTable,
) -> Result<Vec<Function>, IndexTableError> {
    let mut all_objects = Vec::new();

    // Merge constants and objects
    for symbol in constants.iter().chain(objects.iter()) {
        let symbol_idx = objects_symbols.try_index(&symbol.symbol())?;

        // Map types to indices
        let mut types = Vec::with_capacity(symbol.ty().len());
        for t in symbol.ty().iter() {
            types.push(type_symbols.try_index(t)?);
        }

        // Create a Function representing the object (no arguments)
        all_objects.push(Function::object(symbol_idx, Type::either(types)));
    }

    Ok(all_objects)
}

/// Builds the list of value domains for each type.
///
/// Each type in the problem gets a `ValueDomain` that contains
/// the indices of all objects of that type.
///
/// # Arguments
/// * `objects` - Slice of `Function` objects (grounded PDDL objects).
/// * `num_types` - Total number of types in the problem (size of `types_symbols`).
///
/// # Returns
/// A `Vec<ValueDomain>` where each index `i` corresponds to a type,
/// and the `ValueDomain` contains the indices of objects belonging to that type.
///
/// # Example
/// ```rust
/// let types_domains = build_types_domains(&objects, num_types);
/// for (ty_idx, domain) in types_domains.iter().enumerate() {
///     println!("Type {}: {}", ty_idx, domain);
/// }
/// ```
pub fn build_types_domains(objects: &[Function], num_types: usize) -> Vec<ValueDomain> {
    // Initialize a Vec<ValueDomain> with empty domains for each type
    let mut types_domains = vec![ValueDomain::empty(); num_types];

    // For each object, add its index to the corresponding type domain
    for (obj_idx, obj) in objects.iter().enumerate() {
        let ty = obj.ty();
        for ty_idx in ty.iter() {
            types_domains[*ty_idx].add(obj_idx);
        }
    }

    types_domains
}


/*/// Builds all object-fluents from atomic function skeletons using an explicit stack,
/// correctly handling union types (`Type`) for parameters and return type.
/// Adds the created objects to `objects` and updates `types_domains` accordingly.
///
/// # Arguments
/// * `functions` - Slice of `AtomicFunctionSkeleton` (lifted functions)
/// * `types_domains` - Mutable reference to domains of types (`Vec<ValueDomain>`)
/// * `objects` - Mutable reference to vector of grounded objects (`Function`)
/// * `objects_symbols` - Mutable reference to the symbol table for objects
///
/// # Errors
/// Returns `IndexTableError` if symbol insertion fails.
/// Builds all object-fluents from atomic function skeletons using an explicit stack,
/// correctly handling union types (`Type`) for parameters and return type.
/// Adds the created objects to `objects` and updates `types_domains` accordingly.
///
/// # Arguments
/// * `functions` - Slice of `AtomicFunctionSkeleton` (lifted functions)
/// * `types_domains` - Mutable reference to domains of types (`Vec<ValueDomain>`)
/// * `objects` - Mutable reference to vector of grounded objects (`Function`)
/// * `objects_symbols` - Mutable reference to the symbol table for objects
///
/// # Errors
/// Returns `IndexTableError` if symbol insertion fails.
pub fn build_object_fluents(
    functions: &[AtomicFunctionSkeleton],
    types_domains: &mut Vec<ValueDomain>,
    objects: &mut Vec<Function>,
    objects_symbols: &mut IndexTable,
) -> Result<(), IndexTableError> {
    for func in functions {
        // 1 Ne traiter que les fonctions dont le type de retour n'est pas Number
        if func.return_type().is_number() {
            continue;
        }

        // 2 Préparer les domaines effectifs des paramètres
        // Chaque paramètre peut être une union de types, on fait l'union des ValueDomains de ses types primitifs
        let mut param_domains: Vec<ValueDomain> = Vec::with_capacity(func.arity());
        for ty in func.parameters().iter() {
            let mut domain = ValueDomain::new();
            for &primitive_ty_idx in ty.members() {
                // Ajouter tous les objets du type primitif au domaine
                domain.union(&types_domains[primitive_ty_idx]);
            }
            param_domains.push(domain);
        }

        // 3 Utiliser une pile explicite pour générer le produit cartésien
        // Chaque élément de la pile : (niveau_param, indices_courants)
        let mut stack: Vec<(usize, Vec<usize>)> = Vec::new();
        stack.push((0, Vec::new()));

        while let Some((level, current)) = stack.pop() {
            if level == func.parameters().len() {
                //  Tous les paramètres remplis, on peut créer l'objet fluent
                let args = current.clone();

                // Crée un symbole temporaire pour l'objet fluent
                let object_fluent = Function::new(func.name(), &args);

                // Insert dans la table de symboles et récupère l'index
                let obj_idx = objects_symbols.insert(object_fluent);

                // Crée l'objet fluent pour chaque type de retour (union possible)
                for &ret_ty_idx in func.return_type().members() {
                    let object = Function::object(obj_idx, Type::either(vec![ret_ty_idx]));
                    objects.push(object);

                    // Ajoute cet objet au domaine du type de retour
                    types_domains[ret_ty_idx].add(obj_idx);
                }

                continue;
            }

            // 4️⃣ Sinon, pour ce paramètre, empiler toutes les options disponibles
            for &obj_idx in param_domains[level].iter() {
                let mut next = current.clone();
                next.push(obj_idx);
                stack.push((level + 1, next));
            }
        }
    }

    Ok(())
}*/
