use std::collections::HashMap;
use std::collections::HashSet;

use crate::aiplan4rust::syntax::ast::{Ast, AstContent};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::elements::Ident;
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
/// - `ast_old`: A mutable reference to the AST to normalize.
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
/// let mut ast_old = parse_source_code(source)?;
/// normalize_typed_list(&mut ast_old)?; // Required before this step
/// let mut diagnostics = DiagnosticManager::new();
/// let changed = normalize_type_def(&mut ast_old, &mut diagnostics)?;
/// if changed {
///     println!("Merged type declarations.");
/// }
/// ```
pub fn normalize_type_def(
    ast: &mut Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let source = ast.source_name().to_string();

    // Accès immuable au TypedList pour lecture
    let typed_list = match find_types_def_typed_list(ast)? {
        Some(node) => node,
        None => return Ok(false),
    };

    // Clone des enfants pour la passe lecture seule
    let old_typed_items = typed_list.children().to_vec();

    // Première passe : collecte des infos et fusion des TypedItems
    let (merged_items, type_sources, changed) = collect_type_info(old_typed_items)?;

    // Reporting : emprunt immuable d’ast, pas de conflit
    report_warnings(&type_sources, ast, diagnostic_manager)?;

    // Deuxième passe : modification mutable de typed_list
    let typed_list = find_types_def_typed_list(ast)?.ok_or_else(|| {
        ParserInternalError::new("TypedList node disappeared between calls".to_string())
    })?;

    let new_typed_items: Vec<Box<AstNode>> = merged_items.into_values().collect();
    typed_list.set_children(new_typed_items);

    Ok(changed)
}

fn collect_type_info(
    typed_items: Vec<Box<AstNode>>,
) -> Result<(HashMap<Ident, Box<AstNode>>, HashMap<Ident, (HashSet<Ident>, Vec<Span>)>, bool), ParserInternalError> {
    let mut merged_items: HashMap<Ident, Box<AstNode>> = HashMap::new();
    let mut type_sources: HashMap<Ident, (HashSet<Ident>, Vec<Span>)> = HashMap::new();
    let mut changed = false;

    for typed_item in typed_items.into_iter() {
        let (key, ty_opt, type_names, span) = extract_id_and_info(&typed_item)?;

        let entry = type_sources.entry(key).or_insert_with(|| (HashSet::new(), Vec::new()));
        entry.0.extend(type_names.iter().cloned());
        entry.1.push(span.clone());

        changed |= merge_typed_item(&mut merged_items, key, typed_item, ty_opt);
    }

    Ok((merged_items, type_sources, changed))
}

fn report_warnings(
    type_sources: &HashMap<Ident, (HashSet<Ident>, Vec<Span>)>,
    ast: &Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), ParserInternalError> {
    report_implicit_either_type_warnings(type_sources, ast, diagnostic_manager)
}



/// Finds the `TypedList` node inside the `TypesDef` node of the given AST.
///
/// This function searches the root children of the AST for a node of kind `TypesDef`.
/// If found, it returns a mutable reference to its first child, which is expected
/// to be a `TypedList` node.
///
/// # Arguments
///
/// * `ast_old` - A mutable reference to the AST to search.
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
fn extract_id_and_info(
    typed_item: &Box<AstNode>
) -> Result<(Ident, Option<Box<AstNode>>, HashSet<Ident>, Span), ParserInternalError> {
    // Get the children of the typed_item node
    let children = typed_item.children();
    // The first child should be a PrimitiveType node
    let primitive_type_node = &children[0];

    // Check that the node kind is PrimitiveType
    let key = match primitive_type_node.kind() {
        AstKind::PrimitiveType { .. } => {
            // Retrieve the content from the node
            let content = primitive_type_node.content();

            // Match on the content
            match content {
                AstContent::Ident(id) => *id, // Extract the identifier string
                other => {
                    return Err(ParserInternalError::new(format!(
                        "Expected Ident content inside PrimitiveType, got: {:?}", other
                    )));
                }
            }
        }
        other => {
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
        Some(ty_node) => extract_type_ids(ty_node)?,
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
fn extract_type_ids(type_node: &AstNode) -> Result<HashSet<Ident>, ParserInternalError> {
    // Verify the node is of kind Type
    if let AstKind::Type = type_node.kind() {
        let mut ids = HashSet::new();
        // Iterate over children nodes expecting each to be a PrimitiveType
        for child in type_node.children() {
            match child.kind() {
                AstKind::PrimitiveType => {
                    if let AstContent::Ident(id) = child.content() {
                        // Insert the primitive type id (usize) into the set
                        ids.insert(*id);  // Deref to get usize value
                    } else {
                        // Error if the content is not Ident
                        return Err(ParserInternalError::new(format!(
                            "Expected Ident inside PrimitiveType node, found: {:?}",
                            child.content()
                        )));
                    }
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
        Ok(ids)
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
    type_sources: &mut HashMap<Ident, (HashSet<Ident>, Vec<Span>)>,
    key: Ident,
    type_names: &HashSet<Ident>,
    span: &Span,
) {
    // Insert or get the entry for `key` in the map, initializing with empty sets if absent
    let entry = type_sources.entry(key).or_insert_with(|| (HashSet::new(), Vec::new()));

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
    merged_items: &mut HashMap<Ident, Box<AstNode>>,
    key: Ident,
    typed_item: Box<AstNode>,
    ty_opt: Option<Box<AstNode>>,
) -> bool {
    if let Some(existing_item) = merged_items.get_mut(&key) {
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
        merged_items.insert(key, typed_item);
        true
    }
}

/// Reports a warning for each type declared implicitly as an "either type"
/// (i.e., the same type ID is associated with multiple type variants).
///
/// A diagnostic is emitted for each type that has more than one variant
/// declaration location (span), indicating a potential ambiguity.
fn report_implicit_either_type_warnings(
    type_sources: &HashMap<Ident, (HashSet<Ident>, Vec<Span>)>, // Maps each type ID to a set of variant type IDs and their source spans
    ast: &Ast,                                                  // The current AST, used to resolve interned type names
    diagnostic_manager: &mut DiagnosticManager,                 // Where to send diagnostics
) -> Result<(), ParserInternalError> {
    // Iterate over all base types and their associated variant types and locations
    for (ty, (types_set, spans)) in type_sources {
        // Only emit a warning if the same type is declared in more than one place
        if spans.len() > 1 {
            // Get the interned string name for the base type ID (`ty`)
            let ty_name: String = ast
                .expect_str(*ty)
                .ok_or_else(|| ParserInternalError::new(format!(
                    "Missing interned name for type id {ty}"
                )))?
                .to_string();

            // Resolve and collect the interned names for each variant type ID
            let type_names: Vec<String> = types_set
                .iter()
                .map(|id| {
                    ast.expect_str(*id)
                        .map(|s| s.to_string())
                })
                .collect::<Result<Vec<String>, ParserInternalError>>()?;

            // Construct the warning diagnostic with all relevant data
            let diagnostic = Diagnostic::new(
                DiagnosticKind::ImplicitEitherTypeDeclarationWarning {
                    ty: ty_name,              // Name of the base type
                    types: type_names,        // Names of the variant types
                    spans: spans.clone(),     // Source code locations of the declarations
                },
                Provider::Normalizer,         // Marks the normalizer as the source
                ast.source_name().to_string(),// The name of the source file
                spans[0].clone(),             // The primary location for the diagnostic
            );

            // Record the diagnostic
            diagnostic_manager.add_diagnostic(diagnostic);
        }
    }

    Ok(())
}
