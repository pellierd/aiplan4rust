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
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::type_checker::TypeHierarchy;
use crate::aiplan4rust::semantic::{SemanticError, TypeChecker};
use crate::aiplan4rust::syntax::Span;
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
                    if !is_type_symbol_valid(*type_id, type_hierarchy) {
                        no_error = false;

                        // Resolve the precise span for the undeclared type usage
                        let precise_span =
                            resolve_type_usage_span(*type_id, declaration, symbol_table);

                        // Generate an error diagnostic for the user
                        diagnostic_manager.add_diagnostic(Diagnostic::error_undeclared_type(
                            *type_id,
                            declaration.clone(),
                            Provider::Analyzer,
                            context.source_id(),
                            precise_span,
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
///    registered during the domain analysis (covers parents, children, and orphans).
///
/// # Arguments
///
/// * `type_id` - The unique identifier of the symbol being validated as a type.
/// * `type_hierarchy` - A reference to the pre-computed [`TypeHierarchy`]. Using the
///   hierarchy's internal indexing ensures $O(1)$ lookup performance.
///
/// # Returns
///
/// Returns `true` if the symbol is a valid type, `false` otherwise.
fn is_type_symbol_valid(type_id: SymbolId, type_hierarchy: &TypeHierarchy) -> bool {
    // 1. Check if it's a pre-defined PDDL built-in type.
    // We check this first as it's a simple O(1) constant-time check.
    if TypeChecker::is_pddl_builtin_types(type_id) {
        return true;
    }

    // 2. Check if the type is part of the established type hierarchy.
    // Since the hierarchy stores all encountered types (including orphans and
    // those only appearing as parents), this is a definitive O(1) lookup.
    type_hierarchy.contains_type(type_id)
}

/// Resolves the precise [`Span`] of a type usage following a specific declaration.
///
/// In PDDL declarations like `entity - type`, the `Declaration` object often only
/// holds the span of the declared entity. This helper searches the [`SymbolTable`]
/// for the usage of the parent type that is physically closest to and appears
/// after the declaration's span.
///
/// This is particularly useful for diagnostics, allowing the compiler to underline
/// the specific undeclared type instead of the entire declaration line.
///
/// # Arguments
///
/// * `parent_type_id` - The identifier of the type whose span needs to be located.
/// * `declaration` - The declaration acting as the anchor point (e.g., the child entity).
/// * `symbol_table` - The table containing all recorded usages of the symbols.
///
/// # Returns
///
/// Returns the [`Span`] of the nearest usage found after the declaration.
/// If no such usage is found, it falls back to the declaration's own span.
fn resolve_type_usage_span(
    parent_type_id: SymbolId,
    declaration: &Declaration,
    symbol_table: &SymbolTable,
) -> Span {
    let reference_end = declaration.span().end();

    // Retrieve the symbol entry for the parent type (e.g., 'object')
    if let Some(entry) = symbol_table.get_symbol(parent_type_id) {
        let mut closest_span: Option<Span> = None;
        let mut min_distance = usize::MAX;

        // Iterate through all recorded usages for this symbol
        for usage in entry.usages() {
            let usage_start = usage.span().start();

            // We only care about usages appearing AFTER our anchor declaration
            if usage_start >= reference_end {
                let distance = usage_start - reference_end;

                // Track the usage with the minimum distance to the anchor
                if distance < min_distance {
                    min_distance = distance;
                    closest_span = Some(*usage.span());
                }
            }
        }

        // Return the precisely located span if found
        if let Some(span) = closest_span {
            return span;
        }
    }

    // Default fallback: return the declaration's span if no specific usage is resolved
    *declaration.span()
}
