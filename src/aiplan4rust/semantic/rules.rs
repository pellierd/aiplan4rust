use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::symbol::{
    Declaration, Filterable, Scope, SymbolEntry, SymbolKind,
};

/// Version 1 : Comparaison de deux déclarations existantes
pub fn can_share_namespace(existing: &Declaration, new: &Declaration) -> bool {
    check_namespace_compatibility(
        existing.symbol_kind(),
        existing.is_derived(),
        new.symbol_kind(),
        new.is_derived(),
    )
}

/// Version 2 : Vérification d'une déclaration par rapport à un genre attendu
/// (Utile pour ton besoin actuel)
pub fn can_kind_share_namespace(
    existing_kind: SymbolKind,
    existing_is_derived: bool,
    expected_kind: SymbolKind,
) -> bool {
    // On part du principe que le 'expected_kind' n'est pas encore
    // une déclaration dérivée (is_derived = false par défaut)
    check_namespace_compatibility(existing_kind, existing_is_derived, expected_kind, false)
}

fn check_namespace_compatibility(
    kind_a: SymbolKind,
    is_derived_a: bool,
    kind_b: SymbolKind,
    is_derived_b: bool,
) -> bool {
    // 1. CAS DES TYPES IDENTIQUES
    if kind_a == kind_b {
        if kind_a == SymbolKind::Constant {
            return true;
        }
        if kind_a == SymbolKind::Predicate {
            return is_derived_a || is_derived_b;
        }
        return false;
    }

    // 2. CAS DES MÉLANGES AUTORISÉS
    match (kind_a, kind_b) {
        (SymbolKind::DomainName, _) | (_, SymbolKind::DomainName) => true,
        (SymbolKind::ProblemName, _) | (_, SymbolKind::ProblemName) => true,

        (SymbolKind::PrimitiveType, SymbolKind::Constant)
        | (SymbolKind::Constant, SymbolKind::PrimitiveType) => true,
        (SymbolKind::PrimitiveType, SymbolKind::Predicate)
        | (SymbolKind::Predicate, SymbolKind::PrimitiveType) => true,

        (SymbolKind::Task, SymbolKind::Action) | (SymbolKind::Action, SymbolKind::Task) => true,

        _ => false,
    }
}

pub fn find_shadowing_candidate<'a>(
    symbol_entry: &'a SymbolEntry,
    kind: SymbolKind,
    scope: &Scope,
) -> Option<&'a Declaration> {
    let mut best_candidate: Option<&'a Declaration> = None;
    for declaration in symbol_entry.declarations() {
        let decl_kind = declaration.symbol_kind();

        // 1. Gardes rapides sur la compatibilité et le genre
        if !check_kind_compatibility(decl_kind, kind) {
            continue;
        }
        if is_structural_mismatch(decl_kind, kind) {
            continue;
        }

        // 2. Garde sur la visibilité (calcul un peu plus coûteux que le simple enum)
        let decl_scope = declaration.scope();
        if !has_global_visibility(declaration.kind()) && !scope.starts_with(decl_scope) {
            continue;
        }

        // 3. Logique de sélection (Shadowing)
        if let Some(current_best) = best_candidate {
            if decl_scope.len() > current_best.scope().len() {
                best_candidate = Some(declaration);
            }
        } else {
            best_candidate = Some(declaration);
        }
    }

    best_candidate
}

/// Détermine si un type de symbole a une visibilité globale en PDDL/HDDL.
/// En PDDL, presque tout est global (accessible depuis le Problem s'il est dans le Domaine),
/// à l'exception des variables qui sont confinées à leur scope local (?x, ?y).
pub fn has_global_visibility(kind: SymbolKind) -> bool {
    match kind {
        // Les variables sont les SEULS éléments strictement locaux au scope
        SymbolKind::Variable
        | SymbolKind::DomainName
        | SymbolKind::ProblemName
        | SymbolKind::TaskID => false,
        // Cas structurels (Noms de domaine/problème ne sont pas des objets de recherche)
        _ => true,
    }
}

/// Règle métier : Définit si un genre d'usage est compatible avec un genre de déclaration.
///
/// En HDDL/PDDL :
/// - Une déclaration 'Task' peut satisfaire un usage 'Action' (et inversement).
/// - Les autres genres (Variable, Constant, etc.) doivent être strictement identiques.
pub fn check_kind_compatibility(decl_kind: SymbolKind, usage_kind: SymbolKind) -> bool {
    // 1. Identité stricte (Predicat == Predicat, Variable == Variable, etc.)
    if decl_kind == usage_kind {
        return true;
    }

    // 2. Règle métier spécifique HDDL : Pont entre Task et Action
    match (decl_kind, usage_kind) {
        (SymbolKind::Task, SymbolKind::Action) => true,
        (SymbolKind::Action, SymbolKind::Task) => true,
        _ => false,
    }
}

/// Règle 2 : Filtrage structurel (Isolation Domain/Problem)
pub fn is_structural_mismatch(decl_kind: SymbolKind, usage_kind: SymbolKind) -> bool {
    !matches!(usage_kind, SymbolKind::DomainName | SymbolKind::ProblemName)
        && matches!(decl_kind, SymbolKind::DomainName | SymbolKind::ProblemName)
}

/// Règle de compatibilité spécifique (Bercher) :
/// Autorise une Task à matcher une Action (Upcasting) même si les types
/// ne sont pas strictement compatibles selon la hiérarchie standard.
///
/// Ordre des arguments : (Ce qui est défini, Ce qui l'appelle)
pub fn allow_implicit_upcast_for_task_matching(
    decl_kind: SymbolKind,  // La définition (ex: Action)
    usage_kind: SymbolKind, // L'utilisation (ex: Task)
) -> bool {
    decl_kind == SymbolKind::Action && usage_kind == SymbolKind::Task
}

/// Returns true if the given symbol ID represents a reserved PDDL built-in symbol.
///
/// This is the single source of truth for identifying identifiers reserved by the
/// PDDL standard (e.g., `number`, `total-time`, `?duration`).
///
/// ### Permissive Design
/// To ensure robustness across various PDDL benchmarks (such as IPC04), this check is
/// intentionally permissive: it validates reserved symbols regardless of whether
/// the corresponding `:requirements` are explicitly declared. This prevents
/// blocking symbols during early resolution; requirement compliance is
/// validated in a later dedicated pass.
///
/// # Predefined Symbols Handled
/// - `number`, `total-time`, `total-cost`: Used for fluents and numeric fluents.
/// - `?duration`: Implicit variable for durative actions.
/// - `#t`: Continuous time variable for temporal domains.
///
/// # Returns
/// - `true` if the `SymbolId` matches a pre-allocated PDDL built-in constant.
pub fn is_pddl_builtin_symbol_id(id: SymbolId) -> bool {
    matches!(
        id,
        SymbolInterner::NUMBER_SYMBOL_ID
            | SymbolInterner::DURATION_VARIABLE_SYMBOL_ID
            | SymbolInterner::TOTAL_TIME_SYMBOL_ID
            | SymbolInterner::TOTAL_COST_SYMBOL_ID
            | SymbolInterner::CONTINUOUS_VARIABLE_SYMBOL_ID
    )
}

/// Détermine si un genre de symbole est "atomique".
/// Un symbole atomique est une entité simple qui ne possède pas d'arguments
/// et dont la validité repose uniquement sur son existence et son nom.
pub fn is_nominal_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Constant |
        SymbolKind::Variable |
        SymbolKind::PrimitiveType |
        SymbolKind::TaskID |       // INDISPENSABLE pour task1, task2...
        SymbolKind::DomainName |
        SymbolKind::ProblemName
    )
}
