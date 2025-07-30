//! This module provides semantic checks for declared symbols in the symbol table,
//! specifically targeting detection of duplicated declarations. It integrates with
//! the diagnostic infrastructure to report errors or warnings as needed during analysis.

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::semantic::symbol::{Declaration, Symbol};
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};

use std::collections::{HashMap, HashSet};

/// Entry point for checking declared symbols in the symbol table for semantic issues
/// such as duplicate declarations.
///
/// # Parameters
/// - `context`: A reference to the semantic analysis context, which contains the symbol table and AST.
/// - `diagnostic_manager`: A mutable reference to the diagnostic manager used to record warnings and errors.
///
/// # Returns
/// - `Ok(true)` if no critical errors (e.g., conflicting symbol declarations) were found.
/// - `Ok(false)` if conflicting symbol declarations were found (errors were logged).
/// - `Err(ParserInternalError)` if an internal parser error occurred during the check.
///
/// # Example
/// ```
/// let result = check_declared_symbols(&context, &mut diagnostic_manager)?;
/// if !result {
///     // Handle semantic errors
/// }
/// ```
pub fn check_declared_symbols(
    context: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    check_symbol_declarations(context, diagnostic_manager, None)
}

/// Internal helper function that performs detailed checking for duplicate symbol declarations.
///
/// This function iterates through all symbols in the symbol table and checks for
/// multiple declarations of the same symbol within overlapping or nested scopes.
/// If such duplicates are found, appropriate diagnostics are logged.
/// Certain symbols (e.g., of kind `DomainName` or `ProblemName`) are skipped by default,
/// as they are allowed to have duplicates.
///
/// Optionally, the check can be restricted to a set of specific `SymbolKind`s.
///
/// # Parameters
/// - `context`: The semantic context holding the AST and symbol table.
/// - `diagnostic_manager`: The system used to report diagnostics (warnings/errors).
/// - `kinds_to_check`: An optional set of `SymbolKind`s to restrict the check to specific types of symbols.
///
/// # Returns
/// - `Ok(true)` if no errors were found.
/// - `Ok(false)` if duplicate symbol declarations were detected (errors reported).
/// - `Err(ParserInternalError)` if the operation failed due to internal issues (e.g., unresolved names).
fn check_symbol_declarations(
    context: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
    kinds_to_check: Option<&HashSet<SymbolKind>>,
) -> Result<bool, SemanticCheckError> {
    let mut checked = true;
    let symbol_table = context.symbol_table();

    for symbol in symbol_table.values() {
        let mut seen_scopes: HashMap<Scope, Declaration> = HashMap::new();

        for declaration in symbol.declarations() {
            // Filter by desired kinds, if provided
            if let Some(kinds) = kinds_to_check {
                if !kinds.contains(&declaration.symbol_kind()) {
                    continue;
                }
            }

            // Skip kinds like DomainName or ProblemName
            if skip_declaration(declaration)? {
                continue;
            }

            let ast_entry = context.syntax_tree().get_node(declaration.node_id()).unwrap();
            let current_scope = declaration.scope();

            // Check for an existing declaration in an ancestor scope
            let maybe_conflict = seen_scopes.iter().find(|(s, _)| current_scope.starts_with(s));

            if let Some((conflicting_scope, previous_declaration)) = maybe_conflict {
                let current_kind = declaration.symbol_kind();
                let previous_kind = previous_declaration.symbol_kind();

                // Special case: PrimitiveType and Predicate may share ambiguous names
                if (current_kind == SymbolKind::PrimitiveType && previous_kind == SymbolKind::Predicate)
                    || (current_kind == SymbolKind::Predicate && previous_kind == SymbolKind::PrimitiveType)
                {
                    let name = context.interner().try_resolve_ident(symbol.ident())?;
                    let warning = Diagnostic::new(
                        DiagnosticKind::WarningAmbiguousTypePredicateSymbol {
                            symbol: name.to_string(),
                        },
                        Provider::Analyzer,
                        context.source_name().to_string(),
                        ast_entry.span().clone(),
                    );
                    diagnostic_manager.add_diagnostic(warning);
                } else {
                    checked = false;

                    let scope_index = conflicting_scope.iter().last().unwrap();
                    let scope_node = context.syntax_tree().get_node(*scope_index).unwrap();

                    let error = Diagnostic::new(
                        DiagnosticKind::DuplicatedSymbolDeclarationInScope {
                            symbol: Symbol::new(symbol.ident(), declaration.symbol_kind()),
                            original_declaration: previous_declaration.clone(),
                            conflicting_declaration: declaration.clone(),
                            scope: scope_node.kind(),
                        },
                        Provider::Analyzer,
                        context.source_name().to_string(),
                        ast_entry.span().clone(),
                    );
                    diagnostic_manager.add_diagnostic(error);
                }
            } else {
                // First declaration seen in this scope
                seen_scopes.insert(current_scope.clone(), declaration.clone());
            }
        }
    }

    Ok(checked)
}

/// Determines whether a given symbol declaration should be skipped from duplicate checking.
///
/// Currently skips declarations of kind:
/// - `DomainName`
/// - `ProblemName`
///
/// These kinds are allowed to appear multiple times in a program without being considered
/// as semantic errors.
///
/// # Parameters
/// - `declaration`: The declaration to evaluate.
///
/// # Returns
/// - `Ok(true)` if the declaration should be skipped.
/// - `Ok(false)` otherwise.
/// - `Err(ParserInternalError)` if the operation fails unexpectedly.
fn skip_declaration(declaration: &Declaration) -> Result<bool, SemanticCheckError> {
    if matches!(
        declaration.symbol_kind(),
        SymbolKind::DomainName | SymbolKind::ProblemName
    ) {
        return Ok(true);
    }

    Ok(false)
}
