use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::DiagnosticSource;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantic_analyser::SymbolTable;
use crate::aiplan4rust::semantic_analyser::symbol::Declaration;
use crate::aiplan4rust::semantic_analyser::symbol::SymbolKind;

use std::collections::HashMap;
use indexmap::IndexSet;
use linked_hash_set::LinkedHashSet;

/// Represents a duplicate declaration of a symbol along with its AST location.
/// Tuple: (symbol name, first declaration, second declaration, AST id of first declaration).
type ImplicitEitherDiagnosticsData = Vec<(String, Declaration, Declaration, usize)>;

/// Type alias to improve readability: maps a symbol name to a tuple of
/// (ordered unique types, AST node ID from first declaration).
type ImplicitDeclarationGroup = HashMap<String, (LinkedHashSet<String>, usize)>;

/// Merges duplicated primitive type declarations in the symbol table and emits diagnostics.
///
/// This function processes the symbol table of the provided `AnnotatedSyntaxTree`, merges
/// multiple primitive type declarations per symbol into one, and reports any duplicates
/// through diagnostics. It avoids borrow checker conflicts by decoupling mutation and
/// read-only access into two phases.
///
/// # Borrowing Strategy
/// Rust's borrowing rules prevent simultaneous mutable and immutable borrows of the
/// syntax tree. To work around this safely:
/// 1. **Merging Phase** (mutable borrow):
///    Calls [`collect_duplicated_type_declarations`] to mutate the symbol table,
///    consolidate duplicate primitive declarations, and collect raw diagnostic metadata
///    (without needing the syntax tree).
/// 2. **Diagnostic Phase** (immutable borrow):
///    After the mutable borrow ends, uses [`report_implicit_either_type_declaration_warning`]
///    to generate and add warnings to the `DiagnosticManager`, using the syntax tree
///    immutably to locate source spans.
///
/// # Important Note on Type Lists
/// The merge operation can produce typed lists that still contain duplicate types,
/// for example: `t1 - (either t1 t1)`. Therefore, to ensure a fully normalized state
/// without duplicate types inside typed lists, it is recommended to call
/// [`normalize_typed_list`] **after** this function.
///
/// # Parameters
/// - `syntax_tree`: A mutable reference to the `AnnotatedSyntaxTree` to process.
///   Its symbol table will be modified in-place.
/// - `diagnostic_manager`: The manager that will receive emitted warnings for any
///   duplicated type declarations found.
///
/// # Returns
/// Returns `Ok(true)` if any primitive type declarations were merged and the type hierarchy
/// was normalized (i.e., changed). Returns `Ok(false)` if no changes were needed.
/// Returns a [`ParserInternalError`] if:
/// - The merging process fails unexpectedly (e.g., due to an internal invariant violation).
/// - Emitting diagnostics encounters an unrecoverable condition.
///
/// # Behavior Summary
/// - Consolidates all `SymbolKind::PrimitiveType` declarations in the symbol table.
/// - For any duplicates, merges their types into a single declaration and emits a warning.
/// - Leaves all other declaration kinds unchanged.
///
/// # See Also
/// - [`collect_duplicated_type_declarations`] — collects duplicates without needing AST spans.
/// - [`report_implicit_either_type_declaration_warning`] — emits diagnostics using collected metadata.
/// - [`normalize_typed_list`] — recommended to call after to remove any duplicate types within typed lists.
pub fn normalize_type_declarations(
    syntax_tree: &mut AnnotatedSyntaxTree,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    // Clone the filename from the syntax tree to use in diagnostics later.
    let filename = syntax_tree.filename().clone();

    // Mutably borrow the symbol table from the syntax tree to perform merging.
    let symbol_table = syntax_tree.symbol_table_mut();

    // Call the helper function that merges duplicated declarations and returns:
    // 1) the merged primitive declarations
    // 2) a list of raw diagnostic data tuples to be processed later.
    let (type_declarations, diagnostics) = collect_duplicated_type_declarations(symbol_table)?;

    // After the mutable borrow is released, emit diagnostics by converting raw data into actual
    // warnings, accessing the syntax tree immutably for span info.
    report_implicit_either_type_declaration_warning(diagnostics, syntax_tree, &filename, diagnostic_manager)?;

    // Return true if the type hierarchy was changed (normalized), false otherwise.
    Ok(!type_declarations.is_empty())
}


/// Merges primitive type declarations across all symbols in the symbol table and collects
/// duplicates.
///
/// This function processes every symbol's declaration set within the provided symbol table,
/// consolidating redundant primitive type declarations (e.g., multiple `(:type x - object)`).
/// It uses [`merge_symbol_type_declarations`] internally to perform the per-symbol merging logic,
/// and aggregates both the resulting unique primitive declarations and any metadata
/// required to later emit diagnostics for duplicates.
///
/// # Parameters
/// - `symbol_table`: A mutable reference to the full `SymbolTable`, where each symbol maps to
///   a set of declarations (`IndexSet<Declaration>`) that may contain duplicates.
///
/// # Returns
/// Returns a `Result` containing a tuple:
/// - `HashMap<String, Declaration>`: A map where each key is a symbol name, and the value is
///   the merged primitive declaration for that symbol, if one was found.
/// - `Vec<(String, Declaration, Declaration, usize)>`: A collection of tuples representing
///   duplicate primitive type declarations. Each tuple contains:
///     - the symbol name (`String`)
///     - the original declaration retained after merging
///     - the duplicate declaration that was merged
///     - the AST node ID of the duplicate (used to locate its span for diagnostics)
///
/// # Errors
/// Returns a [`ParserInternalError`] if the merging process fails for any symbol, which is
/// not expected under normal parsing conditions but may indicate internal logic issues.
///
/// # Notes
/// - This function **does not emit diagnostics directly**. It collects all necessary
///   metadata and defers diagnostic creation and reporting to the caller.
/// - Non-primitive declarations in the symbol table are unaffected and preserved.
///
/// # See Also
/// - [`merge_symbol_type_declarations`] — performs the actual merge logic for a single symbol.
fn collect_duplicated_type_declarations(
    symbol_table: &mut SymbolTable,
) -> Result<
    (
        HashMap<String, Declaration>,
        ImplicitEitherDiagnosticsData,
    ),
    ParserInternalError,
> {
    let mut unique_primitive_declarations = HashMap::new();
    let mut diagnostics_data = Vec::new();

    // Iterate over each symbol and merge their primitive type declarations
    for (key, symbol) in symbol_table.iter_mut() {
        let declarations = symbol.declarations_mut();

        // Merge declarations and collect diagnostics related to duplicates
        let (maybe_primitive_decl, mut local_diagnostics) =
            merge_symbol_type_declarations(key, declarations)?;

        // Store merged primitive declarations keyed by symbol name
        if let Some(decl) = maybe_primitive_decl {
            unique_primitive_declarations.insert(key.clone(), decl);
        }

        // Append any diagnostic data gathered during merging
        diagnostics_data.append(&mut local_diagnostics);
    }

    Ok((unique_primitive_declarations, diagnostics_data))
}

/// Reports a diagnostic warning when a symbol has multiple type declarations that can
/// be normalized into an `either` type. For example:
///   `x y - (either A B)`
/// This is useful to guide the user toward more idiomatic PDDL.
///
/// # Parameters
/// - `diagnostics_data`: List of conflicting type declarations.
/// - `syntax_tree`: The annotated syntax tree from parsing.
/// - `filename`: The source file where the issue occurred.
/// - `diagnostic_manager`: The system managing all diagnostics.
///
/// # Returns
/// - `Ok(())` on success, or `ParserInternalError` if AST information is missing.
fn report_implicit_either_type_declaration_warning(
    diagnostics_data: ImplicitEitherDiagnosticsData,
    syntax_tree: &AnnotatedSyntaxTree,
    filename: &str,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), ParserInternalError> {
    // Group declarations by symbol and collect types and source location
    let grouped = group_implicit_either_type_declarations(diagnostics_data);

    for (symbol, (types, ast_id)) in grouped {
        // Find the node in the AST to retrieve its span (source position)
        let node = syntax_tree.get_entry(ast_id).ok_or_else(|| {
            ParserInternalError::new("Missing syntax node for type declaration.".to_string())
        })?;

        // Convert the ordered set of types into a Vec for reporting
        let types_vec: Vec<String> = types.into_iter().collect();

        // Construct the diagnostic object
        let diagnostic = Diagnostic::new(
            DiagnosticKind::ImplicitEitherTypeDeclarationWarning {
                ty: symbol,
                types: types_vec,
            },
            DiagnosticSource::SemanticAnalyzer,
            filename.to_string(),
            node.span().clone(),
        );

        // Report it to the diagnostic manager
        diagnostic_manager.add_diagnostic(diagnostic);
    }

    Ok(())
}

/// Groups duplicate type declarations for the same symbol, preserving the order of type appearance.
///
/// For a symbol `x` declared as both:
///   `x - A`
///   `x - B`
/// We normalize it as:
///   `x - (either A B)`
///
/// # Parameters
/// - `diagnostics_data`: Vector of tuples: (symbol, declaration1, declaration2, ast_id).
///
/// # Returns
/// - A map from symbol to its unique set of declared types and the AST id of its first occurrence.
fn group_implicit_either_type_declarations(
    diagnostics_data: ImplicitEitherDiagnosticsData,
) -> ImplicitDeclarationGroup {
    let mut grouped: ImplicitDeclarationGroup = HashMap::new();

    for (symbol, decl1, decl2, ast_id) in diagnostics_data {
        // Extract types without cloning by taking ownership
        let types1 = decl1.into_types().unwrap_or_default();
        let types2 = decl2.into_types().unwrap_or_default();

        // Insert the symbol if not yet present, along with its original AST id
        let entry = grouped.entry(symbol).or_insert_with(|| (LinkedHashSet::new(), ast_id));

        // Insert all types in insertion order, ignoring duplicates
        for t in types1.into_iter().chain(types2) {
            entry.0.insert(t);
        }
    }

    grouped
}

/// Merges primitive type declarations for a given symbol and collects duplicate diagnostics.
///
/// This function processes all declarations associated with a symbol, consolidating multiple
/// primitive type declarations (e.g., `(:type a b - object)`) into a single merged declaration.
/// If duplicates are found, it collects diagnostic metadata to be used later for warning emission.
/// Non-primitive declarations are left untouched.
///
/// The function is designed to **avoid borrow checker conflicts** by:
/// - collecting diagnostic data without accessing the syntax tree,
/// - deferring diagnostic creation and emission to the caller.
///
/// # Parameters
/// - `symbol`: The name of the symbol whose declarations are being processed.
/// - `declarations`: A mutable reference to an `IndexSet<Declaration>` that contains all
///   declarations for the given symbol.
///
/// # Returns
/// Returns a `Result` containing:
/// - `Option<Declaration>`: The merged primitive type declaration if one was found, otherwise
///   `None`.
/// - `Vec<(String, Declaration, Declaration, usize)>`: A list of diagnostic metadata, where each
///   tuple contains:
///     - the symbol name (`String`)
///     - the first (existing) primitive declaration
///     - the second (duplicate) primitive declaration
///     - the AST node ID of the duplicate declaration (for span lookup)
///
/// # Algorithm
/// 1. Take ownership of the original `declarations` by replacing them with an empty set.
/// 2. Iterate over each declaration:
///    - If it's not a primitive type, reinsert it as-is.
///    - If it's a primitive type:
///       - If no primitive declaration has been merged yet, store it.
///       - Otherwise, treat it as a duplicate:
///           - Record diagnostic metadata.
///           - Merge its type content (if any) into the existing declaration.
/// 3. Reinsert the final merged primitive declaration (if one exists).
/// 4. Update the original `declarations` with the new set.
/// 5. Return the merged declaration and diagnostic data.
///
/// # Errors
/// Returns a `ParserInternalError` only if unexpected invariants are violated (not expected under
/// normal use).
///
/// # Notes
/// - This function does not emit diagnostics directly. It only prepares the data required to do so
///   later.
/// - It assumes that `Declaration::types_mut()` and `Declaration::take_types()` are used to access
///   and move internal type data for merging.

fn merge_symbol_type_declarations(
    symbol: &str,
    declarations: &mut IndexSet<Declaration>,
) -> Result<(Option<Declaration>, Vec<(String, Declaration, Declaration, usize)>), ParserInternalError> {
    // Holds the merged primitive declaration, initially None
    let mut merged_primitive_decl: Option<Declaration> = None;
    // Create a new set to accumulate non-primitive declarations and the final merged one
    let mut new_declarations = IndexSet::with_capacity(declarations.len());
    // Vector to store data needed for diagnostics creation later on, without borrowing syntax_tree
    let mut diagnostics_data = Vec::new();

    // Replace the original declarations with an empty set, taking ownership of the old ones
    let old_declarations = std::mem::take(declarations);

    // Iterate over all old declarations
    for mut declaration in old_declarations {
        // If this declaration is not a primitive type, insert it directly into the new set
        if declaration.kind() != &SymbolKind::PrimitiveType {
            new_declarations.insert(declaration);
            continue; // Move to the next declaration
        }

        // If it is a primitive type declaration
        match &mut merged_primitive_decl {
            // If a merged primitive declaration already exists
            Some(existing_decl) => {
                // Store the information necessary for a diagnostic without directly borrowing
                // syntax_tree
                diagnostics_data.push((
                    symbol.to_string(),    // The symbol name
                    existing_decl.clone(), // The already merged declaration
                    declaration.clone(),   // The duplicate declaration found
                    declaration.ast(),     // AST node id for locating the source
                ));

                // Merge the types from the duplicate declaration into the existing merged
                // declaration
                match existing_decl.types_mut() {
                    Some(existing_types) => {
                        // If the new declaration has types, extend the existing types
                        if let Some(new_types) = declaration.take_types() {
                            existing_types.extend(new_types);
                        }
                    }
                    // Otherwise, replace the existing types with those from the new declaration
                    None => {
                        existing_decl.set_types(declaration.take_types());
                    }
                }
            }
            // If this is the first primitive declaration encountered, set it as the merged one
            None => {
                merged_primitive_decl = Some(declaration);
            }
        }
    }

    // If there is a merged primitive declaration, insert it into the new declarations set
    if let Some(decl) = &merged_primitive_decl {
        new_declarations.insert(decl.clone());
    }

    // Replace the original declarations with the updated set containing merged results
    *declarations = new_declarations;

    // Return the merged primitive declaration (if any) and the list of diagnostic data for later
    // use
    Ok((merged_primitive_decl, diagnostics_data))
}
