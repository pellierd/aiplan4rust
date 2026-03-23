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
//! * **Hierarchical types**: Types declared within a parent-child relationship
//!   (e.g., `truck - vehicle`). These are indexed in the [`TypeHierarchy`].
//! * **Primitive types**: Types explicitly declared in the `:types` section but
//!   potentially "orphans" (no parents or children defined).
//!
//! # Performance Optimization
//!
//! To maintain high performance on domains with thousands of symbols, this module
//! leverages a multi-pass approach:
//! 1. **Extraction**: The [`TypeHierarchy`] is extracted once from the [`SymbolTable`].
//! 2. **Indexing**: The hierarchy provides $O(1)$ lookup for any symbol acting as a
//!    parent or child in the type tree.
//! 3. **Validation**: All subsequent checks in [`check_symbol_types`] use these
//!    pre-computed indexes to avoid redundant table scans.
//!
//! This ensures that the overall complexity of the type-checking pass remains $O(N)$
//! relative to the number of declarations.
//!
//! # Errors
//!
//! If a type is used but cannot be resolved through built-ins, the hierarchy, or
//! root declarations, an [`error_undeclared_type`](crate::diagnostic::Diagnostic::error_undeclared_type)
//! is emitted through the [`DiagnosticManager`].

use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Provider};
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::type_checker::TypeHierarchy;
use crate::aiplan4rust::semantic::{SemanticError, TypeChecker};
use crate::SymbolTable;

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
/// * `type_hierarchy` - A pre-computed [`TypeHierarchy`] used for $O(1)$ type validation.
///   Injecting this hierarchy avoids redundant table scans across different check passes.
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
/// By utilizing the provided [`TypeHierarchy`], this function avoids the $O(N)$ cost
/// of re-scanning the symbol table for type definitions. All lookups are performed
/// in constant time, ensuring the validation remains efficient even for large-scale
/// domains with thousands of symbols.
pub fn check_symbol_types(
    context: &CheckContext,
    type_hierarchy: &TypeHierarchy,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    let mut no_error = true;
    let symbol_table = context.symbol_table();

    // Iterate through every symbol stored in the table (constants, types, predicates, etc.)
    for symbol in symbol_table.values() {
        // A symbol can have multiple declarations (e.g., same name in different scopes)
        for declaration in symbol.declarations() {
            // Check if this specific declaration associates a type with the symbol
            // (e.g., in 'v - vehicle', we retrieve the ID for 'vehicle')
            if let Some(type_ids) = declaration.ty() {
                // Iterate through each ID (handles simple types or 'either' unions)
                for type_id in type_ids {
                    // Validate the type via built-ins, the hierarchy, or root declarations
                    if !is_type_symbol_valid(*type_id, symbol_table, type_hierarchy) {
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
/// 2. **Hierarchical**: It is part of the established type hierarchy, specifically
///    acting as a parent (supertype) to other types.
/// 3. **Explicit**: It is explicitly declared as a [`SymbolKind::PrimitiveType`]
///    in the root scope (covers "orphan" types without children or parents).
///
/// # Arguments
///
/// * `type_id` - The unique identifier of the symbol being validated as a type.
/// * `symbol_table` - A reference to the [`SymbolTable`] for root-scope resolution.
/// * `type_hierarchy` - A reference to the pre-computed [`TypeHierarchy`]. Using the
///   hierarchy's internal indexing ensures $O(1)$ lookup performance.
///
/// # Returns
///
/// Returns `true` if the symbol is a valid type, `false` otherwise.
fn is_type_symbol_valid(
    type_id: SymbolId,
    symbol_table: &SymbolTable,
    type_hierarchy: &TypeHierarchy,
) -> bool {
    // 1. Check if it's a pre-defined PDDL built-in type.
    // We check this first as it's a simple O(1) constant-time check.
    if TypeChecker::is_pddl_builtin_types(type_id) {
        return true;
    }

    // 2. Check if the type is used as a parent in the domain.
    // Thanks to the TypeHierarchy index, this is an O(1) lookup.
    if type_hierarchy.is_type_used_as_parent(type_id) {
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
