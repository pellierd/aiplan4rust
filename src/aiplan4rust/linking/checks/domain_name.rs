//! This module provides semantic consistency checks related to the linking phase,
//! specifically verifying relationships and constraints between domain and problem contexts.
//!
//! It includes functions that analyze the linked semantic information, emit diagnostics,
//! and ensure the domain and problem definitions align correctly.
//!
//! # Main functionality
//!
//! - `check_domain_name`: Validates that the domain name declared in the domain AST matches
//!   the domain name referenced in the problem AST, emitting warnings if they differ.
//!
//! # Diagnostics
//!
//! Warnings are emitted through the diagnostic management system to inform about inconsistencies,
//! such as mismatched domain names.
//!
//! # Errors
//!
//! Functions return linking-related errors (e.g., `LinkingError`) if essential declarations are missing
//! or internal inconsistencies are detected.

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Provider};
use crate::aiplan4rust::linking::checks::error::LinkingCheckError;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::SymbolTable;

/// Checks for consistency between the domain name declared in the domain context
/// and the domain name referenced in the problem context.
///
/// This function verifies that a problem is actually linked to the correct domain by
/// comparing their identifiers. If a mismatch is detected, a warning is emitted,
/// but the process continues as PDDL parsers often allow this for flexibility.
///
/// # Parameters
///
/// - `domain_ctx`: The [`CheckContext`] representing the declared domain (the reference).
/// - `problem_ctx`: The [`CheckContext`] representing the problem referencing a domain.
/// - `diags`: A mutable reference to the [`DiagnosticManager`] for collecting
///   mismatch warnings.
///
/// # Returns
///
/// - `Ok(true)`: The check completed successfully (even if names don't match,
///   as a warning is sufficient).
/// - `Err(LinkingCheckError)`: A structural error occurred, such as a missing
///   domain name declaration in either the domain or the problem.
///
/// # Logic
///
/// 1. Resolves the unique `DomainName` symbol in the **domain** context.
/// 2. Resolves the unique `DomainName` symbol in the **problem** context.
/// 3. Compares their internal symbol identifiers.
/// 4. If they differ, emits a `DomainProblemNameMismatch` warning using the
///    problem's source information and the span of the reference.
///
/// [`CheckContext`]: crate::semantics::CheckContext
/// [`DiagnosticManager`]: crate::diagnostics::DiagnosticManager
pub fn check_domain_name(
    domain: &CheckContext,
    problem: &CheckContext,
    domain_symbol_table: &mut SymbolTable,
    problem_symbol_table: &mut SymbolTable,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, LinkingCheckError> {
    // --- 1. Resolve the domain name declared in the domain AST ---
    let declared = domain_symbol_table.try_resolve_unique_declaration(SymbolKind::DomainName)?;

    // --- 2. Resolve the domain name referenced in the problem AST ---
    let referenced = problem_symbol_table.try_resolve_unique_declaration(SymbolKind::DomainName)?;

    // --- 3. Compare both domain names ---
    // If the names don't match, emit a diagnostic warning.
    if declared.symbol().id() != referenced.symbol().id() {
        // --- 4. Retrieve the corresponding AST entry ---
        let ast = problem.syntax_tree().try_node(referenced.source())?;

        // --- 5. Emit a warning about the mismatch ---
        let warning = Diagnostic::warning_domain_problem_name_mismatch(
            declared.clone(),
            referenced.clone(),
            Provider::Linker,
            problem.source(),
            ast.span().clone(),
        );
        diagnostic_manager.add_diagnostic(warning);
    }

    // --- 6. Names match or warning has been emitted; return success ---
    Ok(true)
}
