use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::Usage;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolEntry};
use crate::SymbolTable;

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
    symbol_table: &mut SymbolTable,
    skip_symbols: &[SymbolKind],
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let mut checked = true;
    let mut bindings_to_apply = Vec::new();

    for symbol in symbol_table.values() {
        for usage in symbol.usages().values() {
            let kind = usage.symbol_kind();

            // 1. On garde ton skip_symbol actuel (built-ins + liste d'exclusion)
            if should_skip_symbol(symbol, context, kind, skip_symbols) {
                continue;
            }

            // 3. On cherche la déclaration pour le reste (Action, Variable, Object, etc.)
            if let Some(declaration) = find_declaration(symbol, usage) {
                // 3. VISSAGE SÉLECTIF : Uniquement pour les feuilles sans signature
                // Utilise matches! pour être plus propre et éviter l'erreur de syntaxe
                // Les symbol avec signatures sont binder par check symbol_signature
                if matches!(kind, SymbolKind::Constant | SymbolKind::Variable) {
                    bindings_to_apply.push((symbol.ident(), declaration.source(), usage.source()));
                }
            } else {
                // Aucune déclaration trouvée : Erreur de symbole non déclaré
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

    // --- PHASE 2 : LE VISSAGE (Mutation) ---
    for (symbol_id, decl_node_id, usage_node_id) in bindings_to_apply {
        // On récupère l'entrée mutable pour ce symbole
        let mut entry = symbol_table.try_get_symbol_mut(symbol_id)?;
        // A. Lien Usage -> Declaration
        if let Some(u) = entry.usages_mut().get_mut(&usage_node_id) {
            u.set_resolved_declaration(decl_node_id);
        }
        // B. Lien Declaration -> Usage (Cross-reference)
        if let Some(d) = entry.declarations_mut().get_mut(&decl_node_id) {
            d.add_usage(usage_node_id);
        }
    }

    Ok(checked)
}

/// Determines if a symbol should be skipped during the undeclared symbol check.
///
/// This decision is based on three main criteria:
/// 1. **Structural & Type Isolation**: Structural identifiers (`DomainName`, `ProblemName`)
///    and `PrimitiveType` are skipped to avoid interference with logical symbol resolution.
/// 2. **Predefined Built-ins**: Symbols built into PDDL (e.g., `number`, `total-cost`) are
///    handled natively by the interner and are always considered "declared".
/// 3. **Explicit Exclusion**: Any symbol kind present in the `skip_symbols` list.
///
/// ### Structural Identifiers vs. Logical Symbols
/// `DomainName` and `ProblemName` are treated as structural metadata. They identify
/// PDDL components but do not participate in planning logic (actions, objects, etc.).
/// Skipping them prevents "hijacking" collisions where a logical object shares its name
/// with the domain (e.g., `satellite2`), ensuring the resolver binds the correct entity.
///
/// ### Primitive Types
/// `PrimitiveType` usages are skipped here because they are processed in a dedicated
/// semantic pass (`check_symbol_types`). This separation ensures that type hierarchies
/// and graph-based validation don't clutter the general undeclared symbol check.
///
/// # Arguments
///
/// * `symbol` - The symbol entry to evaluate from the symbol table.
/// * `context` - The current check context, providing access to requirements and AST metadata.
/// * `usage_kind` - The specific kind of the current symbol usage.
/// * `skip_symbols` - A list of symbol kinds to be explicitly ignored during this pass.
///
/// # Returns
///
/// * `true` if the symbol should be skipped, otherwise `false`.
fn should_skip_symbol(
    symbol: &SymbolEntry,
    context: &CheckContext,
    usage_kind: SymbolKind,
    skip_symbols: &[SymbolKind],
) -> bool {
    // 1. Skip structural identifiers (Domain/Problem names) and types.
    // - Domain/Problem names are structural labels, not logical symbols; skipping them
    //   prevents "hijacking" resolution when an object shares its name with the domain.
    // - PrimitiveTypes are ignored here because they are handled by their own dedicated
    //   pass (check_symbol_types) which manages roots and hierarchy.
    if matches!(
        usage_kind,
        SymbolKind::PrimitiveType | SymbolKind::DomainName | SymbolKind::ProblemName
    ) {
        return true;
    }

    // 2. Skip if it's a predefined PDDL built-in (e.g., 'number', 'total-cost').
    // These are reserved by the interner and do not require explicit declaration in the files.
    if is_pddl_builtin_symbol(symbol, context) {
        return true;
    }

    // 3. Skip if the kind is explicitly requested to be ignored by the caller.
    if skip_symbols.contains(&usage_kind) {
        return true;
    }

    false
}

/// Searches for and returns a valid declaration for a given symbol usage within the current context.
///
/// This function identifies which specific declaration associated with a [`SymbolEntry`]
/// governs the provided [`Usage`]. It acts as the primary resolution engine for linking
/// symbol occurrences (like variables, constants, or predicates) to their definitions.
///
/// It validates two main criteria:
/// 1. **Scope Visibility**: The usage must occur within the scope of the declaration
///    (or a sub-scope thereof). In case of shadowed variables, it finds the most relevant
///    declaration allowed by the scope hierarchy.
/// 2. **Namespace Compatibility**: The symbol kind of the declaration must match or be
///    compatible with the kind of the usage (e.g., PDDL requirements for shared namespaces).
///
/// # Parameters
///
/// - `symbol`: The [`SymbolEntry`] containing all known declarations for this identifier.
/// - `usage`: The specific [`Usage`] instance to resolve.
///
/// # Returns
///
/// Returns `Some(&Declaration)` if a matching declaration is found; otherwise returns `None`.
/// Returning the reference allows the caller to perform "binding" (vissage) by storing
/// the declaration's `NodeId` back into the usage.
///
/// # Logic
///
/// The function searches through all declarations of the symbol and returns the first match where:
/// - `usage.scope().starts_with(declaration.scope())`: Ensures the usage is within
///   a legal visibility block.
/// - `decl_kind.can_share_name_space_with(&usage_kind)`: Handles PDDL-specific
///   rules for overlapping namespaces.
///
/// [`SymbolEntry`]: crate::semantics::SymbolEntry
/// [`Usage`]: crate::semantics::Usage
/// [`Declaration`]: crate::semantics::Declaration
fn find_declaration<'a>(symbol: &'a SymbolEntry, usage: &Usage) -> Option<&'a Declaration> {
    let usage_scope = usage.scope();
    let usage_kind = usage.symbol_kind();

    // On utilise .find() pour récupérer la déclaration exacte qui valide l'usage.
    // Cela permet de passer d'une simple vérification d'existence à une phase de résolution.
    symbol.declarations().values().find(|declaration| {
        let decl_kind = declaration.symbol_kind();

        // --- CORRECTION CRUCIALE ---
        // Si l'usage actuel n'est PAS un nom de structure (ex: c'est une Constant ou un Predicate),
        // on ignore les déclarations qui sont des noms de structure.
        // Cela évite que 'satellite2' (Object) ne soit lié à 'satellite2' (Domain).
        if !matches!(usage_kind, SymbolKind::DomainName | SymbolKind::ProblemName)
            && matches!(decl_kind, SymbolKind::DomainName | SymbolKind::ProblemName)
        {
            return false;
        }

        // 1. Le scope de l'usage doit être à l'intérieur du scope de la déclaration
        let scope_match = usage_scope.starts_with(declaration.scope());

        // 2. Le genre doit être compatible (même genre ou partage d'espace de noms)
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
