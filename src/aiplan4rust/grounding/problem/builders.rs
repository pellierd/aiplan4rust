/*use itertools::Itertools;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::problem::{Fluent, SymbolTable, ValueDomain};
use crate::aiplan4rust::lang::ids::{FunctorID, ObjectFluentID, ObjectID, ArgumentID, PredicateID, TypeID};
use crate::aiplan4rust::grounding::problem::object::Object;
use crate::aiplan4rust::grounding::problem::object_fluent::ObjectFluent;
use crate::aiplan4rust::lir::problem::symbol_table::IndexTableError;
use crate::aiplan4rust::lir::problem::LiftedProblem;


/// Builds the value-domain table associated with each typing.
///
/// For every typing, this table contains the set of objects that belong to it,
/// based on the constants and objects declared in the lifted problem.
///
/// # Parameters
/// - `problem`: The lifted problem providing constants and objects.
/// - `objects`: Symbol table mapping object identifiers to `ObjectID`.
/// - `types`: Symbol table mapping typing identifiers to `TypeID`.
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

    for obj in problem.constants().chain(problem.object_symbol_table()) {
        let ty_id = types.try_get_id(&obj.types().members()[0])?;
        let obj_id = objects.try_get_id(&obj.symbol())?;
        type_value_domains_table[ty_id].add_object(obj_id);
    }

    Ok(type_value_domains_table)
}*/
/*pub fn build_object_fluent_type_value_domain(
    type_value_domains_table: &mut [ValueDomain],
    object_fluents_table: &[ObjectFluent],
) {
    for (idx, of) in object_fluents_table.iter().enumerate() {
        let of_id = ObjectFluentID::new(idx);
        type_value_domains_table[of.types().as_usize()].add_object_fluent(of_id);
    }
}*/

/*pub fn build_object_fluents_table(
    problem: &LiftedProblem,
    function_symbols_table: &SymbolTable<FunctorID>,
    type_symbols_table: &SymbolTable<TypeID>,
    type_value_domains_table: &mut Vec<ValueDomain>,
) -> Result<Vec<ObjectFluent>, IndexTableError> {

    let mut object_fluents_table = Vec::new();

    for f in problem.function_skeletons() {
        let name = function_symbols_table.try_get_id(&f.symbol())?;
        let mut parameter_domains: Vec<&[ObjectID]> = Vec::with_capacity(f.parameters().len());

        // Récupère les domaines d'objets pour chaque paramètre
        for ts in f.parameters() {
            let ty_id = type_symbols_table.try_get_id(&ts.types().members()[0])?;
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
}*/

/*pub fn build_fluents_table(
    problem: &LiftedProblem,
    predicate_symbols_table: &SymbolTable<PredicateID>,
    type_symbols_table: &SymbolTable<TypeID>,
    type_value_domains_table: &Vec<ValueDomain>,
) -> Result<Vec<Fluent>, IndexTableError> {

    let mut fluents_table = Vec::new();

    // Pour chaque prédicat lifté
    for p in problem.atom_skeletons() {
        let predicate = predicate_symbols_table.try_get_id(&p.symbol())?;
        let mut parameter_domains = Vec::with_capacity(p.parameters().len());

        // Récupère les ValueDomain pour chaque paramètre
        for ts in p.parameters() {
            let ty_id = type_symbols_table.try_get_id(&ts.types().members()[0])?;
            let vd = &type_value_domains_table[ty_id];
            // Collecte les ParameterID pour ce typing (objets + object-fluents)
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
}*/



/*/// Builds all object-fluents from atomic function skeletons using an explicit stack,
/// correctly handling union types (`Type`) for parameters and return typing.
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
/// correctly handling union types (`Type`) for parameters and return typing.
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
        // 1 Ne traiter que les fonctions dont le typing de retour n'est pas Number
        if func.return_type().is_number() {
            continue;
        }

        // 2 Préparer les domaines effectifs des paramètres
        // Chaque paramètre peut être une union de types, on fait l'union des ValueDomains de ses types primitifs
        let mut param_domains: Vec<ValueDomain> = Vec::with_capacity(func.arity());
        for types in func.parameters().iter() {
            let mut domain = ValueDomain::new();
            for &primitive_ty_idx in types.members() {
                // Ajouter tous les objets du typing primitif au domaine
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

                // Crée l'objet fluent pour chaque typing de retour (union possible)
                for &ret_ty_idx in func.return_type().members() {
                    let object = Function::object(obj_idx, Type::either(vec![ret_ty_idx]));
                    objects.push(object);

                    // Ajoute cet objet au domaine du typing de retour
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
