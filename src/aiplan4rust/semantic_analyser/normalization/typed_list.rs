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
    // Tracks whether any duplicates were removed during normalization
    let mut normalized = false;

    // Clone filename once for reuse in diagnostic reports
    let filename = syntax_tree.filename().clone();

    // Get mutable access to the symbol table
    let symbol_table = syntax_tree.symbol_table_mut();

    // Iterate over each symbol in the symbol table
    for symbol in symbol_table.values_mut() {
        // Drain all declarations for this symbol to avoid borrow checker issues
        let declarations: Vec<Declaration> = symbol.declarations_mut().drain(..).collect();

        // Collect updated declarations in a new IndexSet
        let mut new_declarations = IndexSet::new();

        // Process each declaration individually
        for mut declaration in declarations {
            // Only process declarations that have typed lists
            if let Some(types) = declaration.types_mut() {
                // Remove and collect duplicates from the type list
                let duplicates = remove_duplicates_by_moving(types);

                // If duplicates were found and removed, emit a warning
                if !duplicates.is_empty() {
                    report_duplicate_types_in_symbol_declaration_warning(
                        &declaration,
                        duplicates,
                        &filename,
                        diagnostic_manager,
                    );
                    normalized = true;
                }
            }

            // Reinsert the (possibly updated) declaration into the new set
            new_declarations.insert(declaration);
        }

        // Replace old declarations with the new (normalized) ones
        *symbol.declarations_mut() = new_declarations;
    }

    // Indicate whether normalization changed anything
    Ok(normalized)
}
/// Reports a diagnostic warning when duplicate types are found in a typed list declaration.
///
/// Called by normalization logic to inform the user that a declaration had repeated
/// types (e.g., `x y - (either t1 t1)`) which were removed automatically.
///
/// # Parameters
/// - `declaration`: The declaration from which duplicate types were removed.
/// - `duplicates`: The list of duplicate type names that were removed.
/// - `filename`: The filename where the declaration was found.
/// - `diagnostic_manager`: The system that tracks all emitted diagnostics.
fn report_duplicate_types_in_symbol_declaration_warning(
    declaration: &Declaration,
    duplicates: Vec<String>,
    filename: &str,
    diagnostic_manager: &mut DiagnosticManager,
) {
    // Construct the diagnostic warning with detailed metadata
    let diagnostic = Diagnostic::new(
        DiagnosticKind::DuplicateTypesInSymbolDeclarationWarning {
            symbol: declaration.symbol().clone(),
            duplicate_types: duplicates,
        },
        DiagnosticSource::SemanticAnalyzer,
        filename.to_string(),
        declaration.span().clone(),
    );

    // Submit the warning to the diagnostic manager
    diagnostic_manager.add_diagnostic(diagnostic);
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
