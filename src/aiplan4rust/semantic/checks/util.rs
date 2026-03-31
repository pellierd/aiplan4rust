use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolEntry, SymbolKind, Usage};
use crate::aiplan4rust::tree::NodeId;

/// Vérifie si l'usage correspond à la déclaration sur l'identité, le genre et l'arité.
/// Retourne la liste des arguments de l'usage si la signature est compatible.
pub fn match_signature<'a>(declaration: &Declaration, usage: &'a Usage) -> Option<&'a [NodeId]> {
    // 1. VÉRIFICATION DE L'IDENTITÉ
    if declaration.symbol().id() != usage.symbol().id() {
        return None;
    }

    // 2. VÉRIFICATION DE LA COMPATIBILITÉ DES KINDS (Règle Métier)
    // On utilise la fonction externe pour autoriser Task <-> Action
    if !check_kind_compatibility(declaration.symbol().kind(), usage.symbol().kind()) {
        return None;
    }

    // 3. VÉRIFICATION STRUCTURELLE (Arité)
    let decl_args = declaration.argument_sources();
    let usage_args = usage.argument_sources();

    match (decl_args, usage_args) {
        // Cas avec arguments : on vérifie que les tailles sont identiques
        (Some(d), Some(u)) if d.len() == u.len() => Some(u),
        // Cas sans arguments : c'est un match (on retourne une slice vide)
        (None, None) => Some(&[]),
        // Tous les autres cas (déséquilibre ou un seul None) sont des échecs
        _ => None,
    }
}

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

/*pub fn resolve_declaration<'a>(
    symbol: &'a SymbolEntry,
    usage: &AstNode, // On passe le nœud direct
    scope: &Scope,   // Le scope reste nécessaire
) -> Option<&'a Declaration> {
    // On déduit le SymbolKind à partir du type de nœud AST
    let usage_kind = match usage.kind() {
        AstKind::Variable => SymbolKind::Variable,
        AstKind::Object => SymbolKind::Constant,
        AstKind::Function | AstKind::AtomicFormula => SymbolKind::Function,
        AstKind::DomainName | AstKind::ProblemName => SymbolKind::DomainName,
        _ => return None, // Ou un genre par défaut si nécessaire
    };

    symbol.declarations().values().find(|declaration| {
        let decl_kind = declaration.symbol_kind();

        // --- FILTRAGE DES STRUCTURES ---
        if !matches!(usage_kind, SymbolKind::DomainName | SymbolKind::ProblemName)
            && matches!(decl_kind, SymbolKind::DomainName | SymbolKind::ProblemName)
        {
            return false;
        }

        // 1. Visibilité
        if !usage_scope.starts_with(declaration.scope()) {
            return false;
        }

        // 2. Compatibilité (Genre identique ou Namespace partagé Task/Action)
        decl_kind == usage_kind || decl_kind.can_share_name_space_with(&usage_kind)
    })
}*/
