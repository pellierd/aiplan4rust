use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::DiagnosticSource;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantic_analyser::symbol::Declaration;

use indexmap::IndexSet;
use std::collections::HashSet;
use std::hash::Hash;

/// Normalizes typed lists in all declarations by removing duplicate types and emitting warnings.
///
/// This function iterates over all symbols and their declarations in the symbol table
/// of the given `AnnotatedSyntaxTree`. For each declaration with a typed list,
/// it removes duplicate types to ensure type lists are normalized (no repeated types).
///
/// When duplicates are found and removed, a diagnostic warning is emitted indicating
/// the symbol name and the duplicated types that were removed.
///
/// # Parameters
/// - `syntax_tree`: A mutable reference to the `AnnotatedSyntaxTree` whose symbol table
///   will be updated in-place to normalize typed lists.
/// - `diagnostic_manager`: The manager where duplicate-type warnings will be reported.
///
/// # Returns
/// Returns `Ok(true)` if any typed lists were normalized (duplicates removed).
/// Returns `Ok(false)` if no duplicates were found.
/// Returns a [`ParserInternalError`] if an unexpected internal error occurs.
///
/// # Behavior Summary
/// - Scans all declarations of all symbols for typed lists.
/// - Removes duplicate types from these lists.
/// - Emits a diagnostic warning per declaration where duplicates were removed.
///
/// # See Also
/// - [`normalize_type_declarations`] — which merges duplicated primitive type declarations.
/// - [`remove_duplicates_by_moving`] — utility to remove duplicate types in a list.
pub fn normalize_typed_list(
    syntax_tree: &mut AnnotatedSyntaxTree,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    // Flag to track if any normalization (duplicate removals) occurred
    let mut normalized = false;

    // Clone filename once to reuse in diagnostics
    let filename = syntax_tree.filename().clone();

    // Get a mutable reference to the symbol table to update declarations
    let symbol_table = syntax_tree.symbol_table_mut();

    // Iterate over each symbol in the symbol table
    for symbol in symbol_table.values_mut() {
        // Extract all declarations for this symbol, draining to avoid borrowing conflicts
        let declarations: Vec<Declaration> = symbol.declarations_mut().drain(..).collect();

        // Prepare a new set of declarations after normalization
        let mut new_declarations = IndexSet::new();

        // Process each declaration independently
        for mut declaration in declarations {
            // If the declaration has a typed list, attempt to remove duplicate types
            if let Some(types) = declaration.types_mut() {
                // Remove duplicate types and collect the duplicates that were removed
                let duplicates = remove_duplicates_by_moving(types);

                // If duplicates were found and removed, emit a diagnostic warning
                if !duplicates.is_empty() {
                    let diagnostic = Diagnostic::new(
                        DiagnosticKind::DuplicateTypesInSymbolDeclarationWarning {
                            symbol: symbol.name().clone(),
                            duplicate_types: duplicates,
                        },
                        DiagnosticSource::SemanticAnalyzer,
                        filename.clone(),
                        declaration.span().clone(),
                    );

                    // Add the diagnostic to the diagnostic manager
                    diagnostic_manager.add_diagnostic(diagnostic);

                    // Mark that a normalization has occurred
                    normalized = true;
                }
            }
            // Insert the (possibly modified) declaration back into the new set
            new_declarations.insert(declaration);
        }

        // Replace the symbol's declarations with the normalized set
        *symbol.declarations_mut() = new_declarations;
    }

    // Return whether any normalization was performed
    Ok(normalized)
}

/// Removes duplicates from the given vector by moving elements.
///
/// This function takes ownership of the items in the input vector by draining it,
/// separates unique elements from duplicates without cloning,
/// then replaces the input vector with the unique elements,
/// and returns a vector containing the duplicates.
///
/// # Parameters
/// - `items`: A mutable reference to a vector of elements implementing `Eq` and `Hash`.
///
/// # Returns
/// A vector containing the elements that were duplicates and removed from `items`.
///
/// # Example
/// ```
/// let mut data = vec![1, 2, 2, 3, 4, 4];
/// let duplicates = remove_duplicates_by_moving(&mut data);
/// assert_eq!(data, vec![1, 2, 3, 4]);
/// assert_eq!(duplicates, vec![2, 4]);
/// ```
fn remove_duplicates_by_moving<T>(items: &mut Vec<T>) -> Vec<T>
where
    T: Eq + Hash,
{
    let mut seen = HashSet::new();
    let mut uniques = Vec::new();
    let mut duplicates = Vec::new();

    // Drain all items from the input vector, taking ownership of each element.
    for item in items.drain(..) {
        // Check if the item was not seen before.
        if !seen.contains(&item) {
            seen.insert(item);  // Move the unique item into the HashSet.
        } else {
            duplicates.push(item);  // Move duplicate items into the duplicates vector.
        }
    }

    // Collect unique items from the HashSet into a vector.
    uniques.extend(seen.into_iter());

    // Replace the input vector's contents with the unique items.
    *items = uniques;

    // Return the duplicates vector.
    duplicates
}
