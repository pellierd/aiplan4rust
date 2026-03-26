use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::symbol::SymbolEntry;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::Usage;

/// Checks for undeclared symbols within the syntax tree and reports missing declarations.
///
/// This function iterates through all symbol usages stored in the symbol table and
/// verifies that each one has at least one valid declaration within its visible scope.
///
/// # Specific Handling
///
/// - **Exclusions**: Symbols provided in `skip_symbols` (like domain or problem names)
///   are ignored.
/// - **Types**: `PrimitiveType` usages are skipped here because they are validated
///   by a dedicated pass ([`check_type_hierarchy`]).
/// - **Built-ins**: Built-in PDDL symbols are automatically handled by the
///   `should_skip_symbol` logic.
///
/// # Parameters
///
/// - `context`: A reference to the [`CheckContext`] providing access to the symbol table,
///   syntax tree, and diagnostic metadata.
/// - `skip_symbols`: A slice of [`SymbolKind`] that should be explicitly ignored
///   (e.g., top-level identifiers that don't require formal declarations).
/// - `diagnostic_manager`: A mutable reference to the [`DiagnosticManager`] where
///   undeclared symbol errors are recorded.
///
/// # Returns
///
/// - `Ok(true)`: All relevant symbol usages have matching declarations.
/// - `Ok(false)`: One or more undeclared symbols were detected and reported.
/// - `Err(SemanticCheckError)`: An internal error occurred during the check.
///
/// # Example
///
/// ```rust
/// let check_ctx = context.as_check_context(Provider::Analyzer);
/// let skip = [SymbolKind::DomainName, SymbolKind::ProblemName];
///
/// let is_valid = check_undeclared_symbols(&check_ctx, &skip, &mut diagnostic_manager)?;
/// ```
///
/// [`CheckContext`]: crate::semantics::CheckContext
/// [`SymbolKind`]: crate::semantics::SymbolKind
/// [`DiagnosticManager`]: crate::diagnostics::DiagnosticManager
pub fn check_undeclared_symbols(
    context: &CheckContext,
    skip_symbols: &[SymbolKind],
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let mut checked = true;
    let symbol_table = context.symbol_table();

    for symbol in symbol_table.values() {
        for usage in symbol.usages() {
            let kind = usage.symbol_kind();

            // 1. On garde ton skip_symbol actuel (built-ins + liste d'exclusion)
            if should_skip_symbol(symbol, context, kind, skip_symbols) {
                continue;
            }

            // 2. AJOUT : On ignore aussi les types ici car ils ont leur propre passe
            // (check_symbol_types) qui gère les racines et les parents.
            if kind == SymbolKind::PrimitiveType {
                continue;
            }

            // 3. On vérifie si une déclaration existe pour le reste (Action, Variable, etc.)
            if !is_declaration_found(symbol, usage, context) {
                checked = false;
                let error = Diagnostic::error_undeclared_symbol(
                    usage.clone(),
                    context.provider(),
                    context.source(),
                    usage.span(),
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

/// Checks if a valid declaration exists for a given symbol usage within the current context.
///
/// This function determines if any of the declarations associated with a [`SymbolEntry`]
/// cover the specific [`Usage`]. It validates two main criteria:
/// 1. **Scope Visibility**: The usage must occur within the scope of the declaration
///    (or a sub-scope thereof).
/// 2. **Namespace Compatibility**: The symbol kind of the declaration must match or be
///    compatible with the kind of the usage (e.g., sharing name spaces in PDDL).
///
/// # Parameters
///
/// - `symbol`: The [`SymbolEntry`] containing all known declarations for this identifier.
/// - `usage`: The specific [`Usage`] instance being validated.
/// - `_context`: A reference to the [`CheckContext`] (currently unused, but reserved for
///   future context-aware resolution).
///
/// # Returns
///
/// Returns `true` if at least one declaration matches the usage's scope and kind;
/// otherwise returns `false`.
///
/// # Logic
///
/// The function iterates through all declarations of the symbol and returns `true` if:
/// - `usage.scope().starts_with(declaration.scope())`: Ensures the usage is in a
///   legal visibility block.
/// - `decl_kind.can_share_name_space_with(&usage_kind)`: Handles PDDL-specific
///   rules where different entities might share names or overlap.
///
/// [`SymbolEntry`]: crate::semantics::SymbolEntry
/// [`Usage`]: crate::semantics::Usage
/// [`CheckContext`]: crate::semantics::CheckContext
fn is_declaration_found(symbol: &SymbolEntry, usage: &Usage, _context: &CheckContext) -> bool {
    let usage_scope = usage.scope();
    let usage_kind = usage.symbol_kind();

    // Pour tous les autres symboles (Action, Task, Predicate, Variable, Constant...)
    symbol.declarations().iter().any(|declaration| {
        let decl_kind = declaration.symbol_kind();

        // 1. Le scope de l'usage doit être à l'intérieur du scope de la déclaration
        let scope_match = usage_scope.starts_with(declaration.scope());

        // 2. Le genre doit être compatible via ta méthode de partage d'espace de noms
        let kind_match =
            decl_kind == usage_kind || decl_kind.can_share_name_space_with(&usage_kind);

        scope_match && kind_match
    })
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
        SymbolInterner::NUMBER_SYMBOL_ID
        | SymbolInterner::DURATION_VARIABLE_SYMBOL_ID
        | SymbolInterner::TOTAL_TIME_SYMBOL_ID
        | SymbolInterner::TOTAL_COST_SYMBOL_ID
        | SymbolInterner::CONTINUOUS_VARIABLE_SYMBOL_ID => true,

        _ => false,
    }
}
