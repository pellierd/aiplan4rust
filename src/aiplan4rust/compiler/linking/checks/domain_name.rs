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

use crate::aiplan4rust::compiler::linking::checks::error::LinkingCheckError;
use crate::aiplan4rust::compiler::semantic::checks::CheckContext;
use crate::aiplan4rust::support::diagnostic::{Diagnostic, DiagnosticManager, Provider};
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
    _domain: &CheckContext,
    problem: &CheckContext,
    domain_symbol_table: &SymbolTable,  // Now immutable
    problem_symbol_table: &SymbolTable, // Now immutable
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, LinkingCheckError> {
    // --- 1. Resolve domain names from both tables ---
    // We use specialized O(n) getters that avoid Vec allocations and cloning.
    // In the problem table, we also look for "DomainName" as it is the
    // identifier of the domain the problem claims to belong to.
    let declared = domain_symbol_table.try_domain_name()?;
    let referenced = problem_symbol_table.try_domain_name()?;

    // --- 2. Compare both domain name identifiers ---
    // We compare SymbolId for maximum performance.
    if declared.symbol().id() != referenced.symbol().id() {
        // --- 3. Retrieve the AST node for diagnostic positioning ---
        let ast = problem.syntax_tree().try_node(referenced.source())?;

        // --- 4. Emit a warning about the name mismatch ---
        // Cloning only occurs here, in the cold error path, to populate the diagnostic.
        let warning = Diagnostic::warning_domain_problem_name_mismatch(
            declared.clone(),
            referenced.clone(),
            Provider::Linker,
            problem.source(),
            ast.span().clone(),
        );
        diagnostic_manager.report(warning);
    }

    // --- 5. Return success (warnings do not stop the linking process) ---
    Ok(true)
}
