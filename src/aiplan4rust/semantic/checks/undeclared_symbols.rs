use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolEntry;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::Usage;

/// Checks if there are any undeclared symbols used in the given syntax arena.
///
/// This function scans all symbol usages in the provided `AnnotatedSyntaxTree`
/// and verifies that each usage has at least one valid declaration. If any usage
/// lacks a corresponding declaration, an `UndeclaredSymbolError` diagnostic
/// is generated and recorded in the `DiagnosticManager`.
///
/// Symbol kinds listed in the `skip_symbols` slice (e.g., intentionally undeclared
/// symbols like domain or problem names) are ignored.
///
/// # Parameters
/// - `ast_old`: Reference to the `AnnotatedSyntaxTree` containing symbols and AST.
/// - `skip_symbols`: Slice of `SymbolKind` to be ignored during undeclared symbol checking.
/// - `source`: The `DiagnosticSource` indicating the analysis phase performing the check.
/// - `diagnostic_manager`: Mutable reference to the `DiagnosticManager` where diagnostics are stored.
///
/// # Returns
/// - `Ok(true)` if no undeclared symbols were found.
/// - `Ok(false)` if undeclared symbols were detected (no internal errors occurred).
/// - `Err(ParserInternalError)` if an internal error occurs during checking.
///
/// # Example
/// ```rust
/// let result = check_undeclared_symbols(
///     &annotated_syntax_tree,
///     &[SymbolKind::ProblemName, SymbolKind::DomainName],
///     DiagnosticSource::SemanticAnalyzer,
///     &mut diagnostic_manager,
/// );
///
/// if let Ok(true) = result {
///     println!("No undeclared symbols found.");
/// } else {
///     println!("Undeclared symbols detected.");
/// }
/// ```
pub fn check_undeclared_symbols(
    context: &CheckContext,
    skip_symbols: &[SymbolKind],
    provider: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let mut checked = true;

    let symbol_table = context.symbol_table();

    // Iterate over each symbol in the symbol table.
    for symbol in symbol_table.values() {
        // Iterate over all usages of the symbol.
        for usage in symbol.usages() {
            // Skip the symbol if it meets the criteria (e.g., already declared or needs to be skipped).
            if should_skip_symbol(symbol, context, usage.symbol_kind(), skip_symbols) {
                continue;
            }

            // Check if the declaration for the symbol was found.
            if !is_declaration_found(symbol, usage, context) {
                checked = false;
                let error = Diagnostic::error_undeclared_symbol(
                    usage.clone(),
                    provider,
                    context.source_id(),
                    usage.span().clone(),
                );
                diagnostic_manager.add_diagnostic(error);
            }
        }
    }

    Ok(checked)
}

/// Determines if a symbol should be skipped during the undeclared symbol check.
///
/// This decision is based on whether the symbol is predefined in PDDL (according to the
/// requirements) or if the symbol's kind matches any entry in the `skip_symbols` list.
///
/// The function checks if the symbol is one of the predefined symbols in the Planning Domain
/// Definition Language (PDDL) or if the symbol’s usage kind matches an entry in the provided
/// `skip_symbols` list. The function also takes into account the requirements of the problem
/// as specified in the `annotated_syntax_tree` (such as whether Typing or NumericFluents are
/// required).
///
/// # Arguments
///
/// * `symbol` - The symbol to check. This is typically a symbol from the symbol table that may
///   be used in the program or logic being analyzed.
/// * `annotated_syntax_tree` - A reference to the `AnnotatedSyntaxTree` that provides access to
///   the problem’s requirements and other metadata affecting symbol definitions.
/// * `usage_kind` - The kind of symbol usage, which determines the context in which the symbol
///   is being used, such as a task, action, or primitive type_checker.
/// * `skip_symbols` - A list of symbol kinds (e.g., `SymbolKind::Action`) that should be
///   skipped during the check.
///
/// # Returns
///
/// * `true` if the symbol should be skipped (either because it is a predefined PDDL symbol
///   or its kind is in the `skip_symbols` list), otherwise `false`.
///
/// # Example
///
/// ```rust
/// let should_skip = should_skip_symbol(
///     &symbol,
///     &ast,
///     &SymbolKind::Action,
///     &[SymbolKind::Action],
/// );
/// assert_eq!(should_skip, true);  // Assuming the symbol kind matches and is in the skip list.
/// ```
fn should_skip_symbol(
    symbol: &SymbolEntry,
    context: &CheckContext,
    usage_kind: SymbolKind,
    skip_symbols: &[SymbolKind],
) -> bool {
    // Skip if the symbol is predefined in PDDL or if it matches a symbol kind in the skip list.
    is_pddl_builtin_symbol(symbol, context) || skip_symbols.contains(&usage_kind)
}

/// Checks if a declaration for the given symbol usage exists in the symbol's declarations.
/// This function handles different `SymbolKind`s and checks if the corresponding declaration
/// matches the usage within the given scope.
///
/// The function verifies whether a declaration for the symbol exists that matches the scope
/// and kind of the given usage. Special handling is provided for certain kinds of symbols,
/// such as `PrimitiveType` and `Task`, to allow for cases where the declaration may be implicit
/// or when certain symbols (like tasks or primitive types) can be used in special ways in PDDL.
///
/// # Arguments
///
/// * `symbol` - The symbol to check for declaration. This is the symbol that has potential
///   declarations and usages in the symbol table.
/// * `usage` - The usage of the symbol that needs to be checked. This indicates the context
///   in which the symbol is being used, including the scope and kind of usage.
///
/// # Returns
///
/// * `bool` - Returns `true` if a matching declaration was found for the symbol usage,
///   otherwise returns `false`.
///
/// # Example
///
/// ```rust
/// let declaration_found = is_declaration_found(&symbol, &usage);
/// assert_eq!(declaration_found, true);  // Assuming a matching declaration was found.
/// ```
fn is_declaration_found(symbol: &SymbolEntry, usage: &Usage, context: &CheckContext) -> bool {
    let usage_scope = usage.scope();
    let usage_kind = usage.symbol_kind();

    // --- LOGIQUE UNIFIÉE ---
    // On vérifie si une déclaration est compatible avec l'usage
    let check_declarations = |declaration: &Declaration| {
        let decl_kind = declaration.symbol_kind();

        // 1. Le scope doit correspondre
        let scope_match = usage_scope.starts_with(&declaration.scope());

        // 2. Le genre doit être compatible (soit identique, soit autorisé par can_share)
        // Note: On utilise `decl_kind == usage_kind` ou `can_share`
        // car can_share renvoie false si les genres sont identiques (sauf Constant).
        let kind_match =
            decl_kind == usage_kind || decl_kind.can_share_name_space_with(&usage_kind);

        scope_match && kind_match
    };

    // Pour PrimitiveType, conservation de la logique de "root scope" (PDDL types implicites)
    let check_usages_at_root_scope = |usage: &Usage| {
        let root_id = context.syntax_tree().try_root_id().unwrap();
        let root_scope = Scope::new(root_id, None);
        symbol.usages().iter().any(|u| {
            usage.scope().starts_with(&root_scope) && u.symbol_kind() == usage.symbol_kind()
        })
    };

    match usage_kind {
        // La logique spéciale Task/Action est maintenant absorbée par `can_share_name_space_with`
        // si tu as bien configuré (Task, Action) dans ton match.
        // Sinon, on garde le match spécifique ou on complète `can_share`.
        SymbolKind::Task => symbol.declarations().iter().any(check_declarations), // Nettoyé !

        SymbolKind::PrimitiveType => {
            symbol.declarations().iter().any(check_declarations)
                || symbol
                    .usages()
                    .iter()
                    .any(|u| check_usages_at_root_scope(u))
        }
        _ => symbol.declarations().iter().any(check_declarations),
    }
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
fn is_pddl_builtin_symbol(symbol: &SymbolEntry, _context: &CheckContext) -> bool {
    // We accept these symbols because they are reserved by the interner at initialization.
    // They are considered part of the language's core vocabulary, decoupling symbol
    // existence from requirement-based feature activation.
    match symbol.ident() {
        SymbolInterner::OBJECT_SYMBOL_ID
        | SymbolInterner::NUMBER_SYMBOL_ID
        | SymbolInterner::DURATION_VARIABLE_SYMBOL_ID
        | SymbolInterner::TOTAL_TIME_SYMBOL_ID
        | SymbolInterner::TOTAL_COST_SYMBOL_ID
        | SymbolInterner::CONTINUOUS_VARIABLE_SYMBOL_ID => true,

        _ => false,
    }
}
