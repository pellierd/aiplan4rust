//! Semantic type validation for PDDL/HDDL symbols.
//!
//! This module provides the logic required to verify that every type referenced in a
//! PDDL domain or problem (such as action parameters, constants, or predicate arguments)
//! refers to a legitimately declared type.
//!
//! # Logic & Hierarchy
//!
//! The validator recognizes a type as valid if it fits into one of the following categories:
//! * **Built-in types**: Native PDDL types like `object` or `number`.
//! * **Declared types**: Types explicitly defined in the `:types` section of the domain.
//! * **Implicit types**: Types that appear as parents in the hierarchy, even if they
//!   lack a dedicated declaration line (common in some PDDL dialects).
//!
//! # Performance Optimization
//!
//! Since a domain can contain thousands of symbols, this module avoids nested loops by:
//! 1. Pre-calculating a [`HashSet`] of all parent types in $O(N)$ time.
//! 2. Performing all subsequent validation checks in $O(1)$ time.
//!
//! This ensures that the complexity of the type-checking pass remains linear relative
//! to the number of declarations in the [`SymbolTable`].
//!
//! # Errors
//!
//! If a type is used but not found in the table or built-ins, an
//! [`error_undeclared_type`](Diagnostic::error_undeclared_type) is emitted through
//! the [`DiagnosticManager`].

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Provider};
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::{SemanticError, TypeChecker};
use crate::SymbolTable;
use std::collections::HashSet;

/// Validates the semantic consistency of all type references within the symbol table.
///
/// This function performs a comprehensive pass over every symbol declaration to ensure that
/// any assigned types (e.g., in constants, variables, or predicates) refer to valid,
/// declared types within the PDDL/HDDL domain.
///
/// # Arguments
///
/// * `context` - A reference to the [`CheckContext`] providing access to the current
///   [`SymbolTable`] and source information.
/// * `diagnostic_manager` - A mutable reference to the [`DiagnosticManager`] where
///   any detected "Undeclared Type" errors will be recorded.
///
/// # Returns
///
/// * `Ok(true)` if all type references are valid.
/// * `Ok(false)` if one or more undeclared types were found (diagnostics will be emitted).
/// * `Err(SemanticError)` if a critical internal error occurs during symbol resolution.
///
/// # Performance Note
///
/// This function implements an $O(N)$ pre-computation step via [`collect_parent_types`]
/// to cache the type hierarchy. This ensures that subsequent type lookups are $O(1)$,
/// maintaining high performance even for domains with thousands of symbols.
pub fn check_symbol_types(
    context: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    let mut no_error = true;
    let symbol_table = context.symbol_table();

    // PERFORMANCE CRITICAL: We collect the set of all parent types once.
    // This transforms what would be an O(N) scan inside the loop into a O(1) lookup.
    // This "local cache" is vital for maintaining speed on large PDDL/HDDL domains.
    let parent_types = symbol_table.collect_parent_types();

    // Iterate through every symbol stored in the table (constants, types, predicates, etc.)
    for symbol in symbol_table.values() {
        // A symbol can have multiple declarations (e.g., same name in different scopes)
        for declaration in symbol.declarations() {
            // Check if this specific declaration associates a type with the symbol
            // (e.g., in 'v - vehicle', we retrieve the ID for 'vehicle')
            match declaration.ty() {
                Some(type_ids) => {
                    // Iterate through each ID (handles simple types or 'either' unions)
                    for type_id in type_ids {
                        // Validate the type via built-ins, the parent cache, or root declarations
                        if !is_type_symbol_valid(*type_id, symbol_table, &parent_types) {
                            no_error = false;

                            // Generate an error diagnostic for the user
                            diagnostic_manager.add_diagnostic(Diagnostic::error_undeclared_type(
                                *type_id,
                                declaration.clone(),
                                Provider::Analyzer,
                                context.source_id(),
                                declaration.span().clone(),
                            ));
                        }
                    }
                }
                None => {
                    // If the declaration has no associated type (e.g., the Domain name itself),
                    // we skip it as it is perfectly valid.
                    continue;
                }
            }
        }
    }

    // Returns true if no typing errors were detected
    Ok(no_error)
}

/// Internal helper to verify the validity of a specific [`SymbolId`] as a type.
///
/// A type identifier is considered semantically valid if it satisfies at least one
/// of the following conditions (checked in order of performance cost):
///
/// 1. **Built-in**: It is a reserved PDDL/HDDL type (e.g., `object`, `number`).
/// 2. **Hierarchical**: It is used as a parent in a type declaration (found in `parent_types`).
/// 3. **Explicit**: It is explicitly declared as a [`SymbolKind::PrimitiveType`] in the root scope.
///
/// # Arguments
///
/// * `type_id` - The unique identifier of the symbol being validated as a type.
/// * `symbol_table` - A reference to the [`SymbolTable`] for root-scope resolution.
/// * `parent_types` - A pre-computed [`HashSet`] of all symbols currently acting as parents.
///   Passing this by reference is mandatory to ensure $O(1)$ lookup performance.
///
/// # Returns
///
/// Returns `true` if the symbol is a valid type, `false` otherwise.
fn is_type_symbol_valid(
    type_id: SymbolId,
    symbol_table: &SymbolTable,
    parent_types: &HashSet<SymbolId>, // Passed by reference to avoid costly clones
) -> bool {
    // 1. Check if it's a pre-defined PDDL built-in type.
    // We check this first as it's a simple O(1) constant-time check.
    if TypeChecker::is_pddl_builtin_types(type_id) {
        return true;
    }

    // 2. Check if the type is used as a parent in the domain.
    // Thanks to the local cache (parent_types), this is an O(1) hash lookup
    // instead of a full table scan.
    if parent_types.contains(&type_id) {
        return true;
    }

    // 3. Check for an explicit 'PrimitiveType' declaration at the root scope.
    // This handles types that might not be parents yet but are correctly declared.
    // This is an O(1) key lookup within the SymbolTable.
    symbol_table
        .resolve_declaration(
            &type_id,
            &SymbolKind::PrimitiveType,
            &symbol_table.root_scope(),
        )
        .is_ok_and(|opt| opt.is_some())
}
