use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::rules::is_pddl_builtin_symbol_id;
use crate::aiplan4rust::semantic::signature_checker::MatchResult;
use crate::aiplan4rust::semantic::symbol::SymbolEntry;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::SemanticError;
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

pub fn check_symbol_usage(
    context: &CheckContext,
    symbol_table: &SymbolTable,
    skip_symbols: &[SymbolKind],
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    let mut no_errors = true;

    for symbol_entry in symbol_table.values() {
        for usage in symbol_entry.usages() {
            // --- 1. FILTRAGE ---
            // On ignore les primitives, les built-ins, et les types structurels (DomainName, etc.)
            if should_skip_symbol(symbol_entry, context, usage.symbol_kind(), skip_symbols) {
                continue;
            }

            // --- 2. VÉRIFICATION DE L'EXISTENCE (Undeclared) ---
            let declaration_id = usage.declaration();

            match declaration_id {
                // CAS A : Le symbole n'a aucune déclaration (Ni locale, ni domaine)
                None => {
                    no_errors = false;
                    diagnostic_manager.add_diagnostic(Diagnostic::error_undeclared_symbol(
                        usage.clone(),
                        context.provider(),
                        context.source(),
                        usage.span(),
                    ));
                    // On s'arrête ici pour cet usage car on ne peut pas vérifier
                    // la signature d'un symbole qui n'existe pas.
                    continue;
                }

                // CAS B : Le symbole est déclaré, on vérifie sa signature
                Some(decl_id) => {
                    // On récupère la déclaration pointée
                    let Some(declaration) = symbol_table.get_declaration(decl_id) else {
                        continue;
                    };

                    // --- 3. VÉRIFICATION DE LA SIGNATURE (Signatures) ---
                    // On récupère le MatchResult stocké par le Resolver
                    if let Some(status) = usage.resolution() {
                        match status {
                            MatchResult::Match => {
                                // Tout est parfait.
                            }

                            MatchResult::UpcastMatch {
                                expected,
                                provided,
                                arg_decl,
                                arg_node_id,
                            } => {
                                // Succès partiel (Type plus général) : Warning
                                let arg_node = context.syntax_tree().try_node(*arg_node_id)?;
                                diagnostic_manager.add_diagnostic(
                                    Diagnostic::warning_task_argument_is_supertype_of_declaration(
                                        arg_decl.clone(),
                                        expected.clone(),
                                        provided.clone(),
                                        context.provider(),
                                        context.source(),
                                        arg_node.span(),
                                    ),
                                );
                            }

                            MatchResult::NoMatch(_failure) => {
                                // Erreur de signature (Arguments invalides) : Error
                                no_errors = false;
                                let usage_node = context.syntax_tree().try_node(usage.source())?;
                                diagnostic_manager.add_diagnostic(
                                    Diagnostic::error_invalid_symbol_signature(
                                        declaration.clone(),
                                        usage.clone(),
                                        context.provider(),
                                        context.source(),
                                        usage_node.span(),
                                    ),
                                );
                            }
                        }
                    }
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
    _context: &CheckContext,
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
    if is_pddl_builtin_symbol_id(symbol.id()) {
        return true;
    }

    // 3. Skip if the kind is explicitly requested to be ignored by the caller.
    if skip_symbols.contains(&usage_kind) {
        return true;
    }

    false
}
