use std::collections::HashMap;
use std::collections::HashSet;

use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::Span;

/// Normalizes type declarations in the AST by merging all `TypedItem` nodes
/// that share the same `PrimitiveType` key.
///
/// This function assumes that:
/// - The AST is already **valid** (i.e., produced by a correct parser and validated).
/// - All `TypedList` nodes have already been **normalized** (via the `normalize_typed_list` pass),
///   meaning that each `TypedItem` contains exactly one element and an optional type.
///
/// It performs the following operations:
/// 1. Locates the `TypedList` node inside the `TypesDef` node of the AST.
/// 2. Extracts all `TypedItem` nodes from the `TypedList`.
/// 3. For each `TypedItem`, extracts:
///     - Its key (`PrimitiveType` string),
///     - Its optional type annotation (`Type` node),
///     - The set of contained type names,
///     - Its source span (for diagnostics).
/// 4. Tracks where each type was declared (to identify duplicates).
/// 5. Merges `TypedItem`s that share the same key by:
///     - Appending the contents of each `Type` node to the existing one if already present.
/// 6. Reports diagnostics if multiple declarations for the same key are detected.
/// 7. Replaces the children of the `TypedList` node with the merged list.
///
/// # Arguments
///
/// - `ast`: A mutable reference to the AST to normalize.
/// - `diagnostic_manager`: A manager to collect and report diagnostic warnings.
///
/// # Returns
///
/// - `Ok(true)` if the AST was modified (i.e., at least one type was merged).
/// - `Ok(false)` if no changes were needed.
/// - `Err(ParserInternalError)` if validation or extraction of any type fails.
///
/// # Errors
///
/// Returns an error if the AST structure does not conform to expectations—
/// for example, if nodes have unexpected kinds or missing children.
///
/// # Notes
///
/// This pass **depends on** a prior run of [`normalize_typed_list`] to guarantee that
/// `TypedItem` nodes are structured correctly. Running this function on unnormalized ASTs
/// may result in unexpected errors or incorrect merging behavior.
///
/// This pass can safely be combined with others (e.g., validation, inference), as long
/// as the `TypedList` normalization is applied first.
///
/// # Example
///
/// ```rust,ignore
/// let mut ast = parse_source_code(source)?;
/// normalize_typed_list(&mut ast)?; // Required before this step
/// let mut diagnostics = DiagnosticManager::new();
/// let changed = normalize_type_def(&mut ast, &mut diagnostics)?;
/// if changed {
///     println!("Merged type declarations.");
/// }
/// ```

pub fn normalize_type_def(
    ast: &mut Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    // Obtain the source name from the AST, used in diagnostic reporting
    let source = ast.source_name().to_string();

    // Locate the TypedList node contained in the TypesDef node of the AST
    let typed_list = match find_types_def_typed_list(ast)? {
        Some(node) => node,
        None => return Ok(false), // No TypesDef present, so nothing to normalize
    };

    // HashMap to accumulate merged TypedItems keyed by their PrimitiveType string
    let mut merged_items: HashMap<String, Box<AstNode>> = HashMap::new();

    // Tracks for each PrimitiveType:
    // - the set of all associated type names found (to detect conflicts)
    // - the spans where declarations occur (for diagnostics)
    let mut type_sources: HashMap<String, (HashSet<String>, Vec<Span>)> = HashMap::new();

    // Tracks whether any modifications occurred during normalization
    let mut changed = false;

    // Take ownership of the current TypedList children (TypedItems)
    let old_typed_items = std::mem::take(typed_list.children_mut());

    // Process each TypedItem one by one
    for typed_item in old_typed_items.into_iter() {
        // Extract the key, optional type annotation, type names, and span from the TypedItem
        let (key, ty_opt, type_names, span) = extract_key_and_info(&typed_item)?;

        // Update the tracking of type declarations and source locations
        update_type_sources(&mut type_sources, &key, &type_names, &span);

        // Merge the current TypedItem into the merged_items map, flag if changes occurred
        changed |= merge_typed_item(&mut merged_items, &key, typed_item, ty_opt);
    }

    // After processing all TypedItems, emit warnings if any key has multiple declarations
    report_implicit_either_type_warnings(&type_sources, diagnostic_manager, &source);

    // Replace the children of TypedList with the merged TypedItems
    let new_typed_items: Vec<Box<AstNode>> = merged_items.into_values().collect();
    typed_list.set_children(new_typed_items);

    // Return whether the AST was changed
    Ok(changed)
}

/// Finds the `TypedList` node inside the `TypesDef` node of the given AST.
///
/// This function searches the root children of the AST for a node of kind `TypesDef`.
/// If found, it returns a mutable reference to its first child, which is expected
/// to be a `TypedList` node.
///
/// # Arguments
///
/// * `ast` - A mutable reference to the AST to search.
///
/// # Returns
///
/// * `Ok(Some(&mut AstNode))` - The mutable reference to the `TypedList` node inside `TypesDef` if found.
/// * `Ok(None)` - If no `TypesDef` node is found in the AST.
/// * `Err(ParserInternalError)` - If the `TypesDef` node does not have a `TypedList` child.
///
/// # Errors
///
/// Returns an error if the `TypesDef` node exists but does not have a child node,
/// which should be a `TypedList`.
pub fn find_types_def_typed_list(
    ast: &mut Ast,
) -> Result<Option<&mut AstNode>, ParserInternalError> {
    // Search root children of the AST for a node of kind `TypesDef`
    let types_def_node = match ast
        .root_mut()
        .children_mut()
        .iter_mut()
        .find(|node| matches!(node.kind(), AstKind::TypesDef))
    {
        Some(node) => node,
        None => return Ok(None), // Return None if no TypesDef node is found
    };

    // Attempt to get the first child of TypesDef, which should be a TypedList node
    let typed_list_node = types_def_node
        .children_mut()
        .get_mut(0)
        .ok_or_else(|| {
            // Return an error if the child is missing
            ParserInternalError::new("TypesDef node must have a TypedList child".to_string())
        })?;

    // Return the found TypedList node wrapped in Some
    Ok(Some(typed_list_node))
}

/// Extracts the key, optional type node, set of type names, and span from a TypedItem AST node.
///
/// This function expects the first child of the `typed_item` to be a `PrimitiveType` node,
/// from which it extracts the key string. It also optionally extracts the second child
/// as the type node (`ty_opt`) and collects all type names from it.
/// The span of the primitive type node is also returned for diagnostic purposes.
///
/// # Arguments
///
/// * `typed_item` - A reference to the boxed AST node representing a TypedItem.
///
/// # Returns
///
/// * `Ok((key, ty_opt, type_names, span))`
///   - `key`: The string key extracted from the `PrimitiveType` node.
///   - `ty_opt`: Optional boxed AST node representing the type information (second child).
///   - `type_names`: Set of all type names extracted from `ty_opt`.
///   - `span`: The span of the primitive type node for source location tracking.
///
/// # Errors
///
/// Returns an error if the first child is not a `PrimitiveType` node, or if extracting type names fails.
fn extract_key_and_info(
    typed_item: &Box<AstNode>
) -> Result<(String, Option<Box<AstNode>>, HashSet<String>, Span), ParserInternalError> {
    // Get the children of the typed_item node
    let children = typed_item.children();
    // The first child should be a PrimitiveType node
    let primitive_type_node = &children[0];

    // Extract the key string by matching on the node kind
    let key = match primitive_type_node.kind() {
        AstKind::PrimitiveType(name) => name.clone(),
        other => {
            // Return error if the first child is not PrimitiveType
            return Err(ParserInternalError::new(format!(
                "Expected PrimitiveType node, got: {:?}", other
            )));
        }
    };

    // Optionally get the second child node representing the type info
    let ty_opt = children.get(1).cloned();
    // Clone the span for diagnostics
    let span = primitive_type_node.span().clone();

    // Extract all type names from the optional type node, or an empty set if none
    let type_names = match &ty_opt {
        Some(ty_node) => extract_type_names(ty_node)?,
        None => HashSet::new(),
    };

    // Return all extracted data as a tuple
    Ok((key, ty_opt, type_names, span))
}

/// Extracts a set of primitive type names from a `Type` AST node.
///
/// This function expects the provided node to be of kind `AstKind::Type`
/// and iterates over its children, which should all be `PrimitiveType` nodes,
/// collecting their names into a `HashSet`.
///
/// # Arguments
///
/// * `type_node` - A reference to the AST node expected to be of kind `Type`.
///
/// # Returns
///
/// * `Ok(HashSet<String>)` containing the names of all primitive types found.
/// * `Err(ParserInternalError)` if the node is not of kind `Type`
///   or if any child is not a `PrimitiveType`.
fn extract_type_names(type_node: &AstNode) -> Result<HashSet<String>, ParserInternalError> {
    // Verify the node is of kind Type
    if let AstKind::Type = type_node.kind() {
        let mut names = HashSet::new();
        // Iterate over children nodes expecting each to be a PrimitiveType
        for child in type_node.children() {
            match child.kind() {
                AstKind::PrimitiveType(name) => {
                    // Insert the primitive type name into the set
                    names.insert(name.clone());
                }
                other => {
                    // Error if a child is not a PrimitiveType
                    return Err(ParserInternalError::new(format!(
                        "Expected PrimitiveType inside Type node, found: {:?}",
                        other
                    )));
                }
            }
        }
        Ok(names)
    } else {
        // Error if the root node is not a Type node
        Err(ParserInternalError::new(format!(
            "Expected Type node, found: {:?}",
            type_node.kind()
        )))
    }
}

/// Updates the `type_sources` map by adding type names and span information for a given key.
///
/// This function inserts the `key` into the map if it doesn't exist, initializing it with
/// an empty set of type names and an empty vector of spans. It then extends the set of
/// type names with the provided `type_names` and appends the `span` to the vector.
///
/// # Arguments
///
/// * `type_sources` - A mutable reference to a HashMap where keys are type keys (strings),
///   and values are tuples containing a set of type names and a list of spans.
/// * `key` - The type key to update in the map.
/// * `type_names` - A set of type names to add to the existing set for this key.
/// * `span` - The span to append to the list of spans associated with the key.
fn update_type_sources(
    type_sources: &mut HashMap<String, (HashSet<String>, Vec<Span>)>,
    key: &str,
    type_names: &HashSet<String>,
    span: &Span,
) {
    // Insert or get the entry for `key` in the map, initializing with empty sets if absent
    let entry = type_sources.entry(key.to_string()).or_insert_with(|| (HashSet::new(), Vec::new()));

    // Extend the set of type names with the new ones
    entry.0.extend(type_names.iter().cloned());

    // Append the span to the vector of spans for this key
    entry.1.push(span.clone());
}

/// Merges a `typed_item` into the `merged_items` map based on the given `key`.
///
/// If an item with the same key already exists in `merged_items`, it merges the new type
/// information (`ty_opt`) into the existing item by extending its children. Otherwise,
/// it inserts the new `typed_item` into the map.
///
/// # Arguments
///
/// * `merged_items` - Mutable reference to a map from keys to `AstNode`s representing typed items.
/// * `key` - The key identifying the typed item.
/// * `typed_item` - The new typed item to merge or insert.
/// * `ty_opt` - Optional new type node to merge into existing item's children.
///
/// # Returns
///
/// Returns `true` to indicate that a merge or insert operation was performed (indicating a change).
fn merge_typed_item(
    merged_items: &mut HashMap<String, Box<AstNode>>,
    key: &str,
    typed_item: Box<AstNode>,
    ty_opt: Option<Box<AstNode>>,
) -> bool {
    if let Some(existing_item) = merged_items.get_mut(key) {
        // If the key already exists, try to merge the new type into the existing one
        if let Some(mut new_ty) = ty_opt {
            let existing_children = existing_item.children_mut();
            if existing_children.len() > 1 {
                // Merge new_ty's children into the existing second child
                existing_children[1].children_mut().extend(new_ty.children_mut().drain(..));
            } else {
                // If no second child exists, push new_ty as a new child
                existing_children.push(new_ty);
            }
        }
        true
    } else {
        // Insert the typed_item if key does not exist yet
        merged_items.insert(key.to_string(), typed_item);
        true
    }
}

/// Reports diagnostics warnings for type keys that have multiple declarations.
///
/// For each key in `type_sources`, if there are multiple spans indicating
/// multiple declarations, it creates and adds a diagnostic warning to the manager.
///
/// # Arguments
///
/// * `type_sources` - A map from type keys to a tuple containing the set of associated types
///   and the list of source code spans where they appear.
/// * `diagnostic_manager` - The diagnostic manager where warnings will be added.
/// * `source` - The source name (e.g., filename) associated with the diagnostics.
fn report_implicit_either_type_warnings(
    type_sources: &HashMap<String, (HashSet<String>, Vec<Span>)>,
    diagnostic_manager: &mut DiagnosticManager,
    source: &str,
) {
    // Iterate over all types and their associated sources/spans
    for (ty, (types_set, spans)) in type_sources {
        // Only report if the type is declared multiple times (more than one span)
        if spans.len() > 1 {
            // Create a diagnostic warning about implicit either type declarations
            let diagnostic = Diagnostic::new(
                DiagnosticKind::ImplicitEitherTypeDeclarationWarning {
                    ty: ty.clone(),
                    types: types_set.iter().cloned().collect(),
                    spans: spans.clone(),
                },
                Provider::Normalizer,
                source.to_string(),
                spans[0].clone(),
            );
            // Add the diagnostic to the manager
            diagnostic_manager.add_diagnostic(diagnostic);
        }
    }
}
