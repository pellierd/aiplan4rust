use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::semantic::checks::CheckContext;
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

/// Checks if a symbol is a predefined PDDL built-in symbol.
///
/// This function identifies symbols that are reserved by the PDDL standard (e.g., `object`,
/// `number`, `?duration`).
///
/// ### Permissive Design
/// To ensure robustness across various PDDL benchmarks (such as IPC04), this check is
/// intentionally permissive: it validates reserved symbols regardless of whether
/// the corresponding `:requirements` are explicitly declared in the domain.
///
/// This prevents blocking semantic errors (like E2013) during the initial symbol
/// resolution phase. Strict compliance with requirements is enforced by a
/// dedicated validation module later in the analysis pipeline.
///
/// # Arguments
/// - `symbol`: The symbol entry from the symbol table to check.
/// - `_context`: The semantic context (currently unused, kept for API consistency).
///
/// # Returns
/// - `true` if the symbol ID matches one of the pre-allocated PDDL built-in constants.
/// - `false` otherwise.
///
/// # Predefined Symbols Handled
/// - `object`: Core type for typing/adl.
/// - `number`, `total-time`, `total-cost`: Used for fluents and numeric fluents.
/// - `?duration`: Implicit variable for durative actions.
/// - `#t`: Continuous time variable for temporal domains.
pub fn is_pddl_builtin_symbol(symbol: &SymbolEntry, _context: &CheckContext) -> bool {
    // We accept these symbols because they are reserved by the interner at initialization.
    // They are considered part of the language's core vocabulary, decoupling symbol
    // existence from requirement-based feature activation.
    match symbol.ident() {
        SymbolInterner::NUMBER_SYMBOL_ID
        | SymbolInterner::DURATION_VARIABLE_SYMBOL_ID
        | SymbolInterner::TOTAL_TIME_SYMBOL_ID
        | SymbolInterner::TOTAL_COST_SYMBOL_ID
        | SymbolInterner::CONTINUOUS_VARIABLE_SYMBOL_ID => true,

        _ => false,
    }
}

/// Détermine si un genre de symbole est "atomique".
/// Un symbole atomique est une entité simple qui ne possède pas d'arguments
/// et dont la validité repose uniquement sur son existence et son nom.
pub fn is_atomic_kind(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Constant | SymbolKind::Variable | SymbolKind::PrimitiveType
    )
}
