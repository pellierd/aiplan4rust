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

use crate::aiplan4rust::compiler::linking::checks::LinkingCheckError;
use crate::aiplan4rust::compiler::semantic::checks::CheckContext;
use crate::aiplan4rust::compiler::semantic::rules::{can_share_namespace, is_structural};
use crate::aiplan4rust::compiler::semantic::symbol::{Declaration, SymbolOrigin};
use crate::aiplan4rust::compiler::semantic::SymbolTable;
use crate::aiplan4rust::support::diagnostic::{Diagnostic, DiagnosticManager, Provider};
use crate::aiplan4rust::support::lang::SymbolId;

/// Checks for conflicting symbol declarations between the domain and the problem.
///
/// This function ensures that symbols declared in the problem do not illegally shadow
/// or conflict with existing declarations in the domain. It serves as the primary
/// cross-context consistency check during the linking phase.
///
/// # Parameters
///
/// - `_domain`: The [`CheckContext`] of the domain (unused, kept for API consistency).
/// - `problem`: The [`CheckContext`] of the problem being linked.
/// - `domain_symbol_table`: The reference [`SymbolTable`] from the domain.
/// - `problem_symbol_table`: The [`SymbolTable`] of the problem to validate.
/// - `diagnostic_manager`: Manager used to report any detected conflicts.
///
/// # Returns
///
/// - `Ok(true)`: All problem symbols are compatible with the domain.
/// - `Ok(false)`: Conflicts were found (diagnostics have been emitted).
/// - `Err(LinkingCheckError)`: An internal error occurred during traversal.
///
/// # Logic
///
/// The check follows a high-performance two-step approach:
/// 1. **Filtering**: Skips non-problem symbols and structural metadata (e.g., domain names).
/// 2. **Fast Path ([`check_cross_conflict`])**: A zero-allocation check to detect
///    namespace violations in the domain's root scope.
/// 3. **Slow Path ([`fetch_cross_declarations`])**: If a conflict is found, matching
///    declarations are collected to build a detailed [`Diagnostic`] error.
///
/// [`CheckContext`]: crate::semantic::checks::CheckContext
/// [`SymbolTable`]: crate::semantic::SymbolTable
/// [`DiagnosticManager`]: crate::aiplan4rust::support::diagnostic::DiagnosticManager
pub fn check_cross_declared_symbols(
    _domain: &CheckContext,
    problem: &CheckContext,
    domain_symbol_table: &SymbolTable,
    problem_symbol_table: &SymbolTable,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, LinkingCheckError> {
    let mut checked = true;

    for symbol in problem_symbol_table {
        for declaration in symbol.declarations() {
            // Initial filter: only process "planning entities" originating from the problem
            if declaration.origin() != SymbolOrigin::Problem
                || is_structural(declaration.symbol_kind())
            {
                continue;
            }

            // STEP 1: Detection (Fast Path - Zero allocation)
            if check_cross_conflict(declaration, symbol.id(), domain_symbol_table) {
                // STEP 2: Collection (Slow Path - Only on error)
                let domain_declarations =
                    fetch_cross_declarations(symbol.id(), domain_symbol_table);

                let error = Diagnostic::error_cross_conflict_symbol_declaration(
                    declaration.clone(),
                    domain_declarations,
                    Provider::Linker,
                    problem.source(),
                    declaration.span().clone(),
                );

                diagnostic_manager.report(error);
                checked = false;
            }
        }
    }

    Ok(checked)
}

/// Performs a fast, zero-allocation check for symbol conflicts between different contexts.
///
/// This function is the "Fast Path" of the cross-declaration linker. It determines if a
/// specific declaration from one table (e.g., the Problem) illegally shadows or conflicts
/// with any existing declarations in the root scope of another table (e.g., the Domain).
///
/// # Arguments
///
/// * `target_declaration` - The declaration being checked for potential conflicts.
/// * `symbol_id` - The unique identifier of the symbol associated with the declaration.
/// * `other_table` - The reference [`SymbolTable`] to check against (usually the domain).
///
/// # Returns
///
/// Returns `true` if a conflict is detected based on namespace sharing rules, `false` otherwise.
///
/// # Logic & Filtering
///
/// A conflict is only considered if the candidate in the `other_table`:
/// 1. Resides in the **Root Scope** (global declarations).
/// 2. Is not a **structural** symbol (skips metadata like `DomainName`).
/// 3. Cannot share a namespace with the `target_declaration` (checked via [`can_share_namespace`]).
///
/// # Performance
///
/// This function is decorated with `#[inline]` and designed to be extremely lightweight:
/// - **Zero Allocations**: It operates entirely on references and iterators.
/// - **Short-circuiting**: Uses [`.any()`](Iterator::any) to return as soon as the first
///   conflict is found.
///
/// It should be used as a guard before calling more expensive collection functions
/// like [`fetch_cross_declarations`].
#[inline]
fn check_cross_conflict(
    target_declaration: &Declaration,
    symbol_id: SymbolId,
    other_table: &SymbolTable,
) -> bool {
    let root_scope = other_table.root_scope();

    other_table
        .iter_declarations(symbol_id)
        .filter(|d| {
            // Même logique : Niveau Root + Non Structurel
            d.scope() == &root_scope && !is_structural(d.symbol().kind())
        })
        .any(|d| !can_share_namespace(target_declaration, d))
}

/// Collects all non-structural declarations for a given symbol from the root scope.
///
/// This function serves as a "Slow Path" helper for the linker. It gathers domain
/// declarations that may conflict with a problem declaration, specifically to provide
/// detailed context in diagnostic error messages.
///
/// # Arguments
///
/// * `symbol_id` - The unique identifier of the symbol to look up.
/// * `table` - A reference to the [`SymbolTable`] (usually the domain's table)
///   to be queried.
///
/// # Returns
///
/// Returns a `Vec<Declaration>` containing clones of all matching declarations.
/// A declaration matches if it:
/// 1. Belongs to the root scope of the provided table.
/// 2. Is not a structural symbol (e.g., skips `DomainName` or `ProblemName`).
///
/// # Performance
///
/// This function performs memory allocations and clones the underlying declarations.
/// To optimize the linking process, it should only be called after a conflict has
/// been detected via a zero-allocation check (like `check_cross_conflict`).
#[inline]
fn fetch_cross_declarations(symbol_id: SymbolId, table: &SymbolTable) -> Vec<Declaration> {
    let root_scope = table.root_scope();

    table
        .iter_declarations(symbol_id)
        .filter(|d| d.scope() == &root_scope && !is_structural(d.symbol().kind()))
        .cloned()
        .collect()
}
