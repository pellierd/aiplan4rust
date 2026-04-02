use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::rules::is_pddl_builtin_symbol;
use crate::aiplan4rust::semantic::symbol::SymbolEntry;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::{DiagnosticManager, SymbolTable};

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
///
///

/*pub fn check_undeclared_symbols(
    context: &CheckContext,
    symbol_table: &SymbolTable, // Peut être immutable maintenant !
    skip_symbols: &[SymbolKind],
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let mut no_errors = true;

    for symbol_entry in symbol_table.values() {
        for usage in symbol_entry.usages().values() {
            let kind = usage.symbol_kind();

            // 1. Filtres habituels (ex: ne pas râler pour 'object' ou 'number')
            if should_skip_symbol(symbol_entry, context, kind, skip_symbols) {
                continue;
            }

            // 2. Le verdict est simple : pas de résolution = erreur
            if usage.resolution().is_none() {
                no_errors = false;

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

    Ok(no_errors)
}*/

pub fn check_undeclared_symbols(
    context: &CheckContext,
    symbol_table: &SymbolTable,
    skip_symbols: &[SymbolKind],
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let mut no_errors = true;

    for symbol_entry in symbol_table.values() {
        for usage in symbol_entry.usages().values() {
            // 1. Filtrage (On ignore les primitives, les built-ins, et ce qui est dans skip_symbols)
            if should_skip_symbol(symbol_entry, context, usage.symbol_kind(), skip_symbols) {
                continue;
            }

            // 2. On s'appuie d'abord sur la présence physique d'une déclaration
            let declaration_id = usage.declaration();
            let match_result = usage.resolution(); // Ton MatchResult (Some(Match) ou Some(NoMatch))

            match declaration_id {
                // CAS 1 : Le symbole est bien "vissé" à une déclaration
                Some(_decl_id) => {
                    // Ici, le symbole EST déclaré.
                    // On vérifie si l'utilisation est sémantiquement correcte.
                    if let Some(res) = match_result {
                        if !res.is_match() {
                            // Le symbole existe, mais la signature est mauvaise.
                            // Pour cette passe "undeclared", on pourrait ne rien faire
                            // et laisser une autre passe gérer les erreurs de types,
                            // OU lever une erreur spécifique ici.

                            /* no_errors = false;
                               let error = Diagnostic::error_signature_mismatch(...);
                               diagnostic_manager.add_diagnostic(error);
                            */
                        }
                    }
                }

                // CAS 2 : Aucune déclaration trouvée (Ni localement, ni dans le domaine)
                None => {
                    // C'est ici la véritable erreur "Undeclared Symbol"
                    no_errors = false;

                    println!(
                        "DEBUG [Not Found]: Symbol '{}' (Kind: {:?}) at {:?}",
                        symbol_entry.ident(),
                        usage.symbol_kind(),
                        usage.span()
                    );

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
    }

    Ok(no_errors)
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
