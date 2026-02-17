//! This module provides functions to detect and report conflicting symbol declarations
//! between the domain and problem semantic contexts during the linking phase.
//!
//! It ensures semantic consistency by verifying that symbols declared in the problem
//! do not conflict with existing declarations in the domain, particularly checking for
//! symbols with the same name but differing kinds.
//!
//! Conflicts detected are reported via the diagnostic system to assist in error tracking
//! and resolution during semantic analysis.
//!
//! # Provided Functions
//!
//! - `check_cross_declared_symbols`: Main function that iterates over problem declarations
//!   and checks for conflicts with domain declarations.
//! - `has_relevant_domain_declarations`: Helper to check if the domain has declarations relevant
//!   to a given symbol name, excluding exempt kinds.
//! - `get_relevant_domain_kinds`: Retrieves kinds of relevant domain declarations for a symbol.
//! - `report_cross_conflict_symbol_error`: Emits a diagnostic error for a detected conflict.
//! - `is_declaration_exempt_from_conflict_check`: Determines if a declaration should be skipped
//!   in conflict checking (e.g., special symbol kinds).
//!
//! # Errors
//!
//! Functions return `LinkingCheckError` on internal failures such as missing declarations
//! or interner resolution errors.
//!
//! # Diagnostics
//!
//! Conflict errors are reported as diagnostics of kind `CrossConflictSymbolDeclarationError`
//! containing details on the conflicting symbol name and kinds.

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Provider};
use crate::aiplan4rust::semantic::{SemanticContext, SymbolTable};
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolKind, SymbolOrigin};
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::linking::checks::LinkingCheckError;

/// Checks for conflicting symbol declarations between the problem and domain syntax trees.
///
/// This function verifies that no symbol declared in the problem conflicts with
/// existing declarations in the domain. Specifically, it detects symbols that share
/// the same name but differ in kind between the problem and domain declarations.
/// When such conflicts are found, diagnostic errors are emitted.
///
/// # Parameters
///
/// - `domain`: Reference to the domain's annotated semantic context.
/// - `problem`: Reference to the problem's annotated semantic context.
/// - `source`: The diagnostic provider identifying the source of diagnostics.
/// - `diagnostic_manager`: Mutable reference to the diagnostic manager where conflict diagnostics
///   are recorded.
///
/// # Returns
///
/// - `Ok(true)` if no conflicting declarations were detected.
/// - `Ok(false)` if conflicts were found and reported.
/// - `Err(LinkingCheckError)` if an internal error occurs during checking.
pub fn check_cross_declared_symbols(
    domain: &SemanticContext,
    problem: &CheckContext,
    provider: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, LinkingCheckError> {
    let mut checked = true;
    let domain_symbol_table = domain.symbol_table();
    let problem_symbol_table = problem.symbol_table();

    // Iterate over all symbols declared in the problem context
    for symbol in problem_symbol_table.values() {
        // Iterate over all declarations of the current symbol
        for declaration in symbol.declarations() {
            // Skip declarations exempt from conflict checks or not originating from the problem context
            if !is_declaration_exempt_from_conflict_check(declaration)
                && declaration.origin() == SymbolOrigin::Problem
            {
                // Check if there are any relevant domain declarations for this symbol
                if has_relevant_domain_declarations(domain_symbol_table, symbol.ident()) {
                    // Retrieve all relevant domain declarations for this symbol
                    let domain_declarations = get_relevant_domain_declarations(domain_symbol_table, symbol.ident());

                    // Check if there exists a domain declaration with the same SymbolKind as the problem declaration
                    let same_kind_exists = domain_declarations
                        .iter()
                        .any(|d| d.symbol_kind() == declaration.symbol_kind());

                    // If no domain declaration of the same kind exists, report a cross-conflict error
                    if !same_kind_exists {
                        let error = Diagnostic::error_cross_conflict_symbol_declaration(
                            declaration.clone(),
                            domain_declarations,
                            provider,
                            problem.source_id(),
                            declaration.span().clone(),
                        );
                        diagnostic_manager.add_diagnostic(error);
                        checked = false;
                    }
                }
            }
        }
    }

    Ok(checked)
}


/// Checks if there are any relevant domain declarations for the given symbol name,
/// excluding declarations exempt from conflict checks.
///
/// # Arguments
///
/// * `domain_symbol_table` - Reference to the domain's `SymbolTable`.
/// * `symbol_name` - The symbol identifier to query.
///
/// # Returns
///
/// `true` if at least one relevant domain declaration exists, `false` otherwise.
fn has_relevant_domain_declarations(
    domain_symbol_table: &SymbolTable,
    symbol_name: SymbolId,
) -> bool {
    domain_symbol_table
        .collect_declarations(Some(&symbol_name), None, Some(&&domain_symbol_table.root_scope()))
        .into_iter()
        .any(|d| !is_declaration_exempt_from_conflict_check(&d))
}

/// Retrieves all relevant domain declarations for a given symbol name from the symbol table.
///
/// This function collects declarations of the specified symbol within the root scope of the domain symbol table,
/// filtering out any declarations that are exempt from conflict checks.
///
/// # Parameters
///
/// - `domain_symbol_table`: Reference to the domain's symbol table.
/// - `symbol_name`: The identifier (`Ident`) of the symbol to retrieve declarations for.
///
/// # Returns
///
/// A vector of cloned `Declaration` instances representing all relevant domain declarations
/// for the specified symbol.
///
/// # Notes
///
/// - Declarations exempt from conflict checks are filtered out.
/// - The returned declarations are clones because the underlying collection returns references.
///
/// # Example
///
/// ```ignore
/// let domain_declarations = get_relevant_domain_declarations(&domain_symbol_table, ident);
/// ```
fn get_relevant_domain_declarations(
    domain_symbol_table: &SymbolTable,
    symbol_name: SymbolId,
) -> Vec<Declaration> {
    domain_symbol_table
        .collect_declarations(Some(&symbol_name), None, Some(&domain_symbol_table.root_scope()))
        .into_iter()
        .filter(|decl| !is_declaration_exempt_from_conflict_check(decl))
        .cloned() // clone because collect_declarations returns references
        .collect()
}

/// Returns `true` if the given declaration is exempt from conflict checks.
///
/// Declarations of kind `DomainName` or `ProblemName` are skipped.
///
/// # Arguments
///
/// * `declaration` - Declaration to check.
///
/// # Returns
///
/// `true` if exempt, otherwise `false`.
fn is_declaration_exempt_from_conflict_check(declaration: &Declaration) -> bool {
    matches!(
        declaration.symbol_kind(),
        SymbolKind::DomainName | SymbolKind::ProblemName
    )
}
