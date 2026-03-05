//! This module provides semantic checks for declared symbols in the symbol table,
//! specifically targeting detection of duplicated declarations. It integrates with
//! the diagnostic infrastructure to report errors or warnings as needed during analysis.

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Provider};
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

            let ast_entry = context.syntax_tree().try_node(declaration.node_id())?;
            let current_scope = declaration.scope();

            // Check for an existing declaration in an ancestor scope
            let maybe_conflict = seen_scopes.iter().find(|(s, _)| current_scope.starts_with(s));

            if let Some((conflicting_scope, previous_declaration)) = maybe_conflict {
                let current_kind = declaration.symbol_kind();
                let previous_kind = previous_declaration.symbol_kind();

                // On demande à la logique centralisée si le partage est possible
                if current_kind.can_share_name_space_with(&previous_kind) {

                    // On ne génère un warning QUE pour le cas ambigu Type/Prédicat
                    let is_type = current_kind == SymbolKind::PrimitiveType || previous_kind == SymbolKind::PrimitiveType;
                    let is_pred = current_kind == SymbolKind::Predicate || previous_kind == SymbolKind::Predicate;

                    if is_type && is_pred {
                        let (predicate_decl, type_decl) = if current_kind == SymbolKind::Predicate {
                            (declaration, previous_declaration)
                        } else {
                            (previous_declaration, declaration)
                        };

                        let warning = Diagnostic::warning_ambiguous_type_predicate_symbol(
                            type_decl.clone(),
                            predicate_decl.clone(),
                            Provider::Analyzer,
                            context.source_id(),
                            ast_entry.span().clone(),
                        );
                        diagnostic_manager.add_diagnostic(warning);
                    }

                // Pour Constant vs Constant (colourfragments) ou Type vs Constant : Silence radio.
                } else {
                    checked = false;

                    let scope_index = conflicting_scope.iter().last().unwrap();
                    let scope_node = context.syntax_tree().get_node(*scope_index).unwrap();

                    let error = Diagnostic::error_duplicated_symbol_declaration_in_scope(
                        Symbol::new(symbol.ident(), declaration.symbol_kind()),
                        previous_declaration.clone(),
                        declaration.clone(),
                        scope_node.kind(),
                        Provider::Analyzer,
                        context.source_id(),
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
