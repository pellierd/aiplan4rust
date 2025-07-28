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

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::semantic::{SemanticContext, SymbolTable};
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolKind, SymbolOrigin};
use crate::aiplan4rust::lang::Ident;
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
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, LinkingCheckError> {
    let mut checked = true;
    let domain_symbol_table = domain.symbol_table();
    let problem_symbol_table = problem.symbol_table();

    for symbol in problem_symbol_table.values() {
        for declaration in symbol.declarations() {
            if !is_declaration_exempt_from_conflict_check(declaration)
                && declaration.origin() == SymbolOrigin::Problem
            {
                if has_relevant_domain_declarations(domain_symbol_table, symbol.ident()) {
                    let domain_kinds = get_relevant_domain_kinds(domain_symbol_table, symbol.ident());
                    let same_kind_exists = domain_kinds.iter().any(|k| *k == declaration.symbol_kind());

                    if !same_kind_exists {
                        report_cross_conflict_symbol_error(
                            declaration,
                            domain_kinds,
                            problem,
                            source,
                            diagnostic_manager,
                        )?;
                        checked = false;
                    }
                };
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
    symbol_name: Ident,
) -> bool {
    domain_symbol_table
        .collect_declarations(Some(&symbol_name), None, Some(&&domain_symbol_table.root_scope()))
        .into_iter()
        .any(|d| !is_declaration_exempt_from_conflict_check(&d))
}

/// Retrieves the kinds of all relevant declarations for a given symbol name
/// from the domain's symbol table, excluding those exempt from conflict checks.
///
/// # Arguments
///
/// * `domain_symbol_table` - Reference to the domain's `SymbolTable`.
/// * `symbol_name` - The symbol identifier to query.
///
/// # Returns
///
/// A vector of `SymbolKind` of relevant declarations.
fn get_relevant_domain_kinds(
    domain_symbol_table: &SymbolTable,
    symbol_name: Ident,
) -> Vec<SymbolKind> {
    domain_symbol_table
        .collect_declarations(Some(&symbol_name), None, Some(&&domain_symbol_table.root_scope()))
        .into_iter()
        .filter(|d| !is_declaration_exempt_from_conflict_check(d))
        .map(|d| d.symbol_kind().clone())
        .collect()
}

/// Reports a conflict error when a problem symbol declaration conflicts with
/// domain declarations.
///
/// # Parameters
///
/// - `declaration`: The conflicting problem `Declaration`.
/// - `domain_kinds`: Kinds of conflicting domain declarations.
/// - `context`: The problem semantic context.
/// - `source`: The diagnostic source provider.
/// - `diagnostic_manager`: Manager to record diagnostics.
///
/// # Returns
///
/// Returns a `Result<(), LinkingCheckError>` indicating success or failure in error reporting.
fn report_cross_conflict_symbol_error(
    declaration: &Declaration,
    domain_kinds: Vec<SymbolKind>,
    context: &CheckContext,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), LinkingCheckError> {
    let symbol = declaration.symbol_ident();
    let symbol_name = context.interner().try_resolve(symbol)?;
    let error = Diagnostic::new(
        DiagnosticKind::CrossConflictSymbolDeclarationError {
            symbol: symbol_name.to_string(),
            problem_kind: declaration.symbol_kind().clone(),
            domain_kinds,
        },
        source,
        context.source_name().to_string(),
        declaration.span().clone(),
    );
    diagnostic_manager.add_diagnostic(error);
    Ok(())
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
