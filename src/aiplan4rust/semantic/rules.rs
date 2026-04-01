use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolEntry, SymbolKind};

pub fn resolve_declaration<'a>(
    symbol: &'a SymbolEntry,
    kind: SymbolKind,
    scope: &Scope,
) -> Option<&'a Declaration> {
    symbol.declarations().values().find(|declaration| {
        let decl_kind = declaration.symbol_kind();

        // 1. Compatibilité (Le plus rapide : simple comparaison d'enums)
        if !check_kind_compatibility(decl_kind, kind) {
            return false;
        }

        // 2. Filtrage des structures (Évite les collisions avec Domain/Problem)
        if is_structural_mismatch(decl_kind, kind) {
            return false;
        }

        // 3. Visibilité (Le plus lent : itération sur les composants du Scope)
        if !scope.starts_with(declaration.scope()) {
            return false;
        }

        true
    })
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
