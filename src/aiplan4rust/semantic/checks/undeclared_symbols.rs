use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::rules::{is_pddl_builtin_symbol, resolve_declaration};
use crate::aiplan4rust::semantic::symbol::SymbolEntry;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
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
            let kind = usage.symbol().kind();

            // 1. On garde ton skip_symbol actuel (built-ins + liste d'exclusion)
            if should_skip_symbol(symbol, context, kind, skip_symbols) {
                continue;
            }

            // 3. On cherche la déclaration pour le reste (Action, Variable, Object, etc.)
            if let Some(declaration) = resolve_declaration(symbol, kind, usage.scope()) {
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
            u.set_declaration(decl_node_id);
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
pub fn should_skip_symbol(
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
