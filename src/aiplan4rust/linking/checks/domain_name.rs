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

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::linking::checks::error::LinkingCheckError;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::semantic::symbol::SymbolKind;

/// Checks for consistency between the domain name declared in the domain AST
/// and the domain name referenced in the problem AST.
///
/// This function performs the following steps:
/// 1. Resolves the domain name declared in the domain file.
/// 2. Resolves the domain name referenced in the problem file.
/// 3. Compares the two names:
///     - If they match, nothing happens.
///     - If they differ, it emits a diagnostic warning indicating the mismatch.
/// 4. If any expected declaration or AST entry is missing, a `LinkingError` is returned.
///
/// # Arguments
///
/// * `domain` - The annotated semantic context representing the domain file.
/// * `problem` - The annotated semantic context representing the problem file.
/// * `source` - The provider of diagnostic source information.
/// * `diagnostic_manager` - The manager responsible for collecting diagnostics.
///
/// # Returns
///
/// * `Ok(true)` if the check completes successfully (regardless of whether names match).
/// * `Err(LinkingCheckError)` if domain name declarations or AST entries are missing.
///
/// # Diagnostics
///
/// Emits a `DomainProblemNameMismatch` warning if the domain names differ.
pub fn check_domain_name(
    domain: &SemanticContext,
    problem: &CheckContext,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, LinkingCheckError> {
    // --- 1. Resolve the domain name declared in the domain AST ---
    let declared = domain.symbol_table().try_resolve_unique_declaration(SymbolKind::DomainName)?;

    // --- 2. Resolve the domain name referenced in the problem AST ---
    let referenced = problem.symbol_table().try_resolve_unique_declaration(SymbolKind::DomainName)?;

    // --- 3. Compare both domain names ---
    // If the names don't match, emit a diagnostic warning.
    if declared.ident() != referenced.ident() {
        // --- 4. Locate the AST syntax for the referenced domain name ---
        let domain_name_declaration = problem.symbol_table().try_resolve_declaration(
            &referenced.ident(),
            &SymbolKind::DomainName,
            &problem.symbol_table().root_scope(),
        )?;

        // --- 5. Retrieve the corresponding AST entry ---
        let ast = problem.syntax_tree().try_node(domain_name_declaration.node_id())?;

        // --- 6. Emit a warning about the mismatch ---
        let domain_name = domain.interner().try_resolve_ident(declared.ident())?;
        let problem_domain_name = problem.interner().try_resolve_ident(referenced.ident())?;
        let warning = Diagnostic::new(
            DiagnosticKind::DomainProblemNameMismatch {
                domain_name: domain_name.to_string(),
                problem_name: problem_domain_name.to_string(),
            },
            source,
            problem.source_name().to_string(),
            ast.span().clone(),
        );
        diagnostic_manager.add_diagnostic(warning);
    }

    // --- 7. Names match or warning has been emitted; return success ---
    Ok(true)
}
