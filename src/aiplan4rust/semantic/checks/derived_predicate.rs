use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolKind};
use crate::aiplan4rust::semantic::{SemanticError, TypeChecker};
use crate::{DiagnosticManager, SymbolTable};

pub fn check_derived_predicates(
    _context: &CheckContext,
    symbol_table: &mut SymbolTable,
    type_checker: &TypeChecker,
    _diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    let mut no_error = true;

    // 1. Collecter les liens (Base <-> Derived) à créer
    // On stocke (SymbolIdent, BaseNodeId, DerivedNodeId)
    let mut links_to_create = Vec::new();

    for entry in symbol_table.values() {
        // On sépare les bases potentielles et les dérivés pour ce symbole
        let mut base_predicates = Vec::new();
        let mut derived_predicates = Vec::new();

        for decl in entry.declarations().values() {
            match decl.symbol_kind() {
                SymbolKind::Predicate => base_predicates.push(decl),
                SymbolKind::DerivedPredicate => derived_predicates.push(decl),
                _ => {}
            }
        }

        // Pour chaque dérivé, on cherche SA signature de base correspondante
        for derived in derived_predicates {
            let mut found = false;
            for base in &base_predicates {
                if match_signatures(base, derived, type_checker)? {
                    links_to_create.push((entry.ident(), base.source(), derived.source()));
                    found = true;
                    break; // On a trouvé le match, on passe au dérivé suivant
                }
            }

            if !found {
                // Optionnel : Gérer ici le cas où un axiome n'a aucune signature valide
                // no_error = false;
            }
        }
    }

    // 2. Application du "vissage" (Mutation de la SymbolTable)
    for (symbol_id, base_id, derived_id) in links_to_create {
        let mut entry = symbol_table.try_get_symbol_mut(symbol_id)?;
        let declarations = entry.declarations_mut();

        // Lien Base -> Derived (Ajout à la liste Vec<NodeId>)
        if let Some(base_decl) = declarations.get_mut(&base_id) {
            base_decl.add_derivation(derived_id);
        }

        // Lien Derived -> Base (Option<NodeId>)
        if let Some(derived_decl) = declarations.get_mut(&derived_id) {
            derived_decl.set_derived_source(base_id);
        }
    }

    Ok(no_error)
}

/// Compare deux déclarations (typiquement une Signature et un Axiome) pour voir si elles matchent.
///
/// Pour qu'il y ait correspondance, les deux déclarations doivent :
/// 1. Avoir le même nombre d'arguments.
/// 2. Pour chaque position, le type de l'argument du `derived` doit être
///    un sous-type (ou identique) au type de l'argument de la `base`.
fn match_signatures(
    base: &Declaration,
    derived: &Declaration,
    type_checker: &TypeChecker,
) -> Result<bool, SemanticError> {
    // 1. Récupération des arguments (doivent être présents pour des prédicats/axiomes)
    let args_base = match base.arguments() {
        Some(args) => args,
        None => return Ok(false),
    };
    let args_derived = match derived.arguments() {
        Some(args) => args,
        None => return Ok(false),
    };

    // 2. Vérification de l'arité (Nombre d'arguments)
    if args_base.len() != args_derived.len() {
        return Ok(false);
    }

    // 3. Vérification positionnelle des types (Sous-typage)
    for (arg_base, arg_derived) in args_base.iter().zip(args_derived.iter()) {
        // Règle standard : le type de l'axiome doit être un sous-type de la signature.
        // Exemple : base(at ?x-vehicle) match derived(at ?x-robot) car robot <: vehicle.
        if !type_checker.is_any_subtype_of(arg_derived.ty(), arg_base.ty())? {
            return Ok(false);
        }
    }

    Ok(true)
}
