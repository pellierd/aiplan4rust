use itertools::Itertools;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::problem::{Fluent, SymbolTable, ValueDomain};
use crate::aiplan4rust::lang::ids::{FunctionID, ObjectFluentID, ObjectID, ArgumentID, PredicateID, TypeID};
use crate::aiplan4rust::grounding::problem::object::Object;
use crate::aiplan4rust::grounding::problem::object_fluent::ObjectFluent;
use crate::aiplan4rust::grounding::problem::symbol_table::IndexTableError;
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
    lifted_problem: &LiftedProblem,
    types_table : &mut SymbolTable<TypeID>,
)  {

    for ts in lifted_problem.types() {
        // Add the symbol's own identifier
        types_table.insert(ts.symbol());
        // Add all member identifiers of the type
        for ty_id in ts.ty().iter() {
            types_table.insert(*ty_id);
        }
    }

}

/// Populates a `SymbolTable` with all function symbols from a `LiftedProblem`.
///
/// Each unique function symbol—either a numeric function or an object-fluent—is inserted into
/// the `SymbolTable` with a unique identifier. This allows efficient lookup of functions by
/// symbol during grounding.
///
/// # Parameters
/// - `problem`: The lifted problem containing all function and object-fluent symbols.
/// - `table`: The `SymbolTable<FunctionID>` to populate.
///
/// # Example
/// ```rust
/// # use aiplan4rust::grounding::problem::{SymbolTable, FunctionID, build_functions_symbols_table};
/// let mut table = SymbolTable::new(Rc::new(StringInterner::new()));
/// build_functions_symbols_table(&lifted_problem, &mut table);
/// ```
pub fn build_functions_symbols_table(
    problem: &LiftedProblem,
    table: &mut SymbolTable<FunctionID>
) {
    for func in problem.functions() {
        table.insert(func.symbol());
    }
}

/// Populates a `SymbolTable` with all predicates from a `LiftedProblem`.
///
/// Each unique predicate is inserted into the `SymbolTable` with a unique identifier,
/// allowing fast lookup of predicates by name or index.
///
/// # Parameters
/// - `problem`: The lifted problem containing the predicates.
/// - `table`: The `SymbolTable<PredicateID>` to populate.
///
/// # Example
/// ```rust
/// # use aiplan4rust::lang::{AtomicFormulaSkeleton, Ident};
/// # use aiplan4rust::grounding::problem::{SymbolTable, PredicateID, build_predicates_symbols_table};
/// let preds = vec![AtomicFormulaSkeleton::new(Ident::new("at"), vec![])];
/// let mut table = SymbolTable::new(Rc::new(StringInterner::new()));
/// build_predicates_symbols_table(&lifted_problem, &mut table);
/// assert!(table.contains(&Ident::new("at")));
/// ```
pub fn build_predicates_symbols_table(
    problem: &LiftedProblem,
    table: &mut SymbolTable<PredicateID>
) {
    for pred in problem.predicates() {
        table.insert(pred.symbol());
    }
}

/// Populates a `SymbolTable` with all objects from a `LiftedProblem`.
///
/// This includes:
/// - Constants defined in the domain.
/// - Objects declared in the problem instance.
///
/// Functions, including object-fluents, are **not** included in this table.
///
/// # Parameters
/// - `problem`: The lifted problem containing constants and objects.
/// - `table`: The `SymbolTable<ObjectID>` to populate.
///
/// # Example
/// ```rust
/// # use aiplan4rust::grounding::problem::{SymbolTable, ObjectID, build_objects_symbols_table};
/// let mut table = SymbolTable::new(Rc::new(StringInterner::new()));
/// build_objects_symbols_table(&lifted_problem, &mut table);
/// ```
pub fn build_objects_symbols_table(
    problem: &LiftedProblem,
    table: &mut SymbolTable<ObjectID>,
) {
    for obj in problem.constants().chain(problem.objects()) {
        table.insert(obj.symbol());
    }
}

/// Builds a table mapping each type to its parent (super) type.
///
/// Each type in the lifted problem is assigned its corresponding parent type ID.
/// This is used to reconstruct the type hierarchy in the grounded problem.
///
/// # Parameters
/// - `problem`: A reference to the `LiftedProblem` containing all types.
/// - `typed_symbols_table`: The `SymbolTable<TypeID>` mapping each type's `Ident` to a unique `TypeID`.
///
/// # Returns
/// A `Vec<TypeID>` where the index corresponds to a `TypeID` and the value at that index
/// is the `TypeID` of its parent (super) type.
/// Returns an `IndexTableError` if a type or its super type cannot be found in the symbol table.
///
/// # Panics
/// This function does not panic; errors are returned via `Result`.
///
/// # Example
/// ```rust
/// # use aiplan4rust::grounding::problem::{build_type_parent_table, SymbolTable, TypeID, IndexTableError};
/// # let lifted_problem = /* ... */ ;
/// # let typed_symbols_table: SymbolTable<TypeID> = /* ... */ ;
/// let parent_table: Result<Vec<TypeID>, IndexTableError> = build_type_parent_table(&lifted_problem, &typed_symbols_table);
/// ```
pub fn build_type_parent_table(
    problem: &LiftedProblem,
    typed_symbols_table: &SymbolTable<TypeID>,
) -> Result<Vec<Option<TypeID>>, GroundingError> {
    let mut types = vec![None; typed_symbols_table.len()];

    for ts in problem.types() {
        let type_id = typed_symbols_table.try_get_id(&ts.symbol())?;
        let members = ts.ty().members();

        if members.len() > 1 {
            return Err(GroundingError::non_flattened_type_error(&ts.ty()));
        }

        if let Some(super_type_symbol) = members.first() {
            let super_type_id = typed_symbols_table.try_get_id(super_type_symbol)?;
            types[type_id] = Some(super_type_id);
        }
    }

    Ok(types)
}

/// Builds the list of objects (constants and declared objects) for the grounded problem.
///
/// Each object is represented as an `Object` with a symbol and its associated type.
/// Constants and objects are merged into a single list, and their types are looked up
/// using the `type_symbols` table.
///
/// # Parameters
/// - `problem`: Reference to the `LiftedProblem` containing constants and objects.
/// - `objects_symbols`: Symbol table mapping each object's identifier to a unique `ObjectID`.
/// - `type_symbols`: Symbol table mapping each type's identifier to a unique `TypeID`.
///
/// # Returns
/// A `Vec<Object>` containing all objects in the problem, with their symbol and type IDs.
///
/// # Errors
/// Returns an `IndexTableError` if an object or its type cannot be found in the corresponding symbol table.
pub fn build_objects_table(
    problem: &LiftedProblem,
    objects_symbols: &SymbolTable<ObjectID>,
    type_symbols: &SymbolTable<TypeID>,
) -> Result<Vec<Object>, IndexTableError> {
    let mut objects = Vec::new();

    // Merge constants and objects
    for object in problem.constants().chain(problem.objects()) {
        let object_id = objects_symbols.try_get_id(&object.symbol())?;
        let type_id = type_symbols.try_get_id(&object.ty().members()[0])?;
        objects.push(Object::new(object_id, type_id));
    }

    Ok(objects)
}

/// Builds the value-domain table associated with each type.
///
/// For every type, this table contains the set of objects that belong to it,
/// based on the constants and objects declared in the lifted problem.
///
/// # Parameters
/// - `problem`: The lifted problem providing constants and objects.
/// - `objects`: Symbol table mapping object identifiers to `ObjectID`.
/// - `types`: Symbol table mapping type identifiers to `TypeID`.
///
/// # Returns
/// A `Result` containing a vector indexed by `TypeID`, where each entry is the
/// corresponding `ValueDomain`, or an `IndexTableError` if any symbol lookup fails.
pub fn build_object_type_value_domains_table(
    problem: &LiftedProblem,
    objects: &SymbolTable<ObjectID>,
    types: &SymbolTable<TypeID>,
) -> Result<Vec<ValueDomain>, IndexTableError> {
    let mut type_value_domains_table = vec![ValueDomain::empty(); types.len()];

    for obj in problem.constants().chain(problem.objects()) {
        let ty_id = types.try_get_id(&obj.ty().members()[0])?;
        let obj_id = objects.try_get_id(&obj.symbol())?;
        type_value_domains_table[ty_id].add_object(obj_id);
    }

    Ok(type_value_domains_table)
}
pub fn build_object_fluent_type_value_domain(
    type_value_domains_table: &mut [ValueDomain],
    object_fluents_table: &[ObjectFluent],
) {
    for (idx, of) in object_fluents_table.iter().enumerate() {
        let of_id = ObjectFluentID::new(idx);
        type_value_domains_table[of.ty().as_usize()].add_object_fluent(of_id);
    }
}

pub fn build_object_fluents_table(
    problem: &LiftedProblem,
    function_symbols_table: &SymbolTable<FunctionID>,
    type_symbols_table: &SymbolTable<TypeID>,
    type_value_domains_table: &mut Vec<ValueDomain>,
) -> Result<Vec<ObjectFluent>, IndexTableError> {

    let mut object_fluents_table = Vec::new();

    for f in problem.functions() {
        let name = function_symbols_table.try_get_id(&f.symbol())?;
        let mut parameter_domains: Vec<&[ObjectID]> = Vec::with_capacity(f.parameters().len());

        // Récupère les domaines d'objets pour chaque paramètre
        for ts in f.parameters() {
            let ty_id = type_symbols_table.try_get_id(&ts.ty().members()[0])?;
            let ty_dom = type_value_domains_table[ty_id].objects();
            parameter_domains.push(ty_dom);
        }

        let return_ty = type_symbols_table.try_get_id(&f.return_type().members()[0])?;

        // Produit cartésien paresseux de toutes les combinaisons
        for combination_refs in parameter_domains.iter().map(|v| v.iter()).multi_cartesian_product() {
            let combination: Vec<ObjectID> = combination_refs.into_iter().cloned().collect();
            let of = ObjectFluent::new(name, combination, return_ty);
            object_fluents_table.push(of);
        }
    }

    Ok(object_fluents_table)
}

pub fn build_fluents_table(
    problem: &LiftedProblem,
    predicate_symbols_table: &SymbolTable<PredicateID>,
    type_symbols_table: &SymbolTable<TypeID>,
    type_value_domains_table: &Vec<ValueDomain>,
) -> Result<Vec<Fluent>, IndexTableError> {

    let mut fluents_table = Vec::new();

    // Pour chaque prédicat lifté
    for p in problem.predicates() {
        let predicate = predicate_symbols_table.try_get_id(&p.symbol())?;
        let mut parameter_domains = Vec::with_capacity(p.parameters().len());

        // Récupère les ValueDomain pour chaque paramètre
        for ts in p.parameters() {
            let ty_id = type_symbols_table.try_get_id(&ts.ty().members()[0])?;
            let vd = &type_value_domains_table[ty_id];
            // Collecte les ParameterID pour ce type (objets + object-fluents)
            let domain: Vec<ArgumentID> = vd.iter_parameters().collect();
            parameter_domains.push(domain);
        }

        // Produit cartésien paresseux de toutes les combinaisons
        for combination_refs in parameter_domains
            .iter()
            .map(|v| v.iter()) // itérateur sur les ParameterID
            .multi_cartesian_product()
        {
            let combination: Vec<ArgumentID> = combination_refs.into_iter().cloned().collect();
            let fluent = Fluent::new(predicate, combination);
            fluents_table.push(fluent);
        }
    }

    Ok(fluents_table)
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

            //  Sinon, pour ce paramètre, empiler toutes les options disponibles
            for &obj_idx in param_domains[level].iter() {
                let mut next = current.clone();
                next.push(obj_idx);
                stack.push((level + 1, next));
            }
        }
    }

    Ok(())
}*/
