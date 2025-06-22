use std::collections::HashMap;
use std::collections::HashSet;

use crate::aiplan4rust::syntax::ast::Ast;
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
    // Étape 1 : traitement mutable (fusion des types)
    let (type_sources, changed) = process_types_def(ast)?;

    // Étape 2 : reporting immuable
    report_implicit_either_type_warnings(&type_sources, ast, diagnostic_manager)?;

    Ok(changed)
}

/// Searches for a `TypedList` node in the AST, collects and merges its `TypedItems`,
/// and updates the `TypedList` with the merged results.
///
/// Returns a map of type sources (used for diagnostics) and a boolean indicating
/// whether any changes were made.
///
/// If no `TypedList` node is found (e.g., for problems that do not declare any `typedef`),
/// the function performs no operation and returns an empty result with `changed = false`.
fn process_types_def(
    ast: &mut Ast,
) -> Result<(HashMap<Ident, (HashSet<Ident>, Vec<Span>)>, bool), ParserInternalError> {
    let Some(typed_list) = find_types_def_typed_list_mut(ast)? else {
        // If no TypedList is found, return empty data and mark no changes
        return Ok((HashMap::new(), false));
    };

    let old_typed_items = typed_list.children().to_vec();

    let (merged_items, type_sources, changed) = collect_type_info(old_typed_items)?;

    typed_list.set_children(merged_items.into_values().collect());

    Ok((type_sources, changed))
}

/// Collects and merges type information from a list of typed AST nodes.
///
/// Processes each typed item by extracting its identifier, optional type node,
/// associated type names, and source span. It accumulates merged typed items,
/// tracks sources of type names and their spans, and records whether any
/// merging changes occurred.
///
/// # Arguments
/// * `typed_items` - A vector of boxed AST nodes representing typed items.
///
/// # Returns
/// * `Ok((merged_items, type_sources, changed))`
///     - `merged_items`: A map from identifiers to their merged typed AST nodes.
///     - `type_sources`: A map from identifiers to a tuple containing:
///         - A set of associated type identifiers.
///         - A vector of spans representing source locations of the types.
///     - `changed`: A boolean indicating if any merging was performed.
///
/// # Errors
/// Returns `ParserInternalError` if extraction of ID and info from any typed item fails.
fn collect_type_info(
    typed_items: Vec<Box<AstNode>>,
) -> Result<(HashMap<Ident, Box<AstNode>>, HashMap<Ident, (HashSet<Ident>, Vec<Span>)>, bool), ParserInternalError> {
    let mut merged_items: HashMap<Ident, Box<AstNode>> = HashMap::new();
    let mut type_sources: HashMap<Ident, (HashSet<Ident>, Vec<Span>)> = HashMap::new();
    let mut changed = false;

    for typed_item in typed_items {
        let (key, ty_opt, type_names, span, typed_item) = extract_id_and_info(typed_item)?;
        update_type_sources(&mut type_sources, key, &type_names, &span);
        changed |= merge_typed_item(&mut merged_items, key, typed_item, ty_opt);
    }

    Ok((merged_items, type_sources, changed))
}

/// Finds the `TypedList` node inside a `TypesDef` node in the AST.
///
/// # Arguments
/// * `ast` - Mutable reference to the AST.
///
/// # Returns
/// * `Ok(Some(&mut AstNode))` - The first `TypedList` node found inside a `TypesDef` node.
/// * `Ok(None)` - If no `TypesDef` node is present in the AST.
/// * `Err(ParserInternalError)` - If a `TypesDef` node is found but it does not contain a `TypedList` child.
pub fn find_types_def_typed_list_mut(
    ast: &mut Ast,
) -> Result<Option<&mut AstNode>, ParserInternalError> {
    let types_def_node = match ast.find_node_of_kind_mut(AstKind::TypesDef) {
        Some(node) => node,
        None => return Ok(None),
    };

    let typed_list_node = types_def_node
        .find_node_of_kind_mut(AstKind::TypedList)
        .ok_or_else(|| ParserInternalError::new("TypesDef node must have a TypedList child".to_string()))?;

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
    mut typed_item: Box<AstNode>, // Ownership du typed_item
) -> Result<(Ident, Option<Box<AstNode>>, HashSet<Ident>, Span, Box<AstNode>), ParserInternalError> {
    let children = typed_item.children_mut();

    if children.is_empty() {
        return Err(ParserInternalError::new(
            "Expected at least one child node for typed_item".to_string(),
        ));
    }

    let primitive_type_node = &children[0];

    if *primitive_type_node.kind() != AstKind::PrimitiveType {
        return Err(ParserInternalError::new(format!(
            "Expected PrimitiveType node, got: {:?}", primitive_type_node.kind()
        )));
    }

    let key = primitive_type_node.try_ident()?;
    let span = primitive_type_node.span().clone();

    // Retire le deuxième enfant s'il existe, sinon None
    let ty_opt = if children.len() > 1 {
        Some(children[1].clone())
    } else {
        None
    };

    // Extraction des identifiants de type à partir de ty_opt
    let type_names = match ty_opt.as_ref() {
        Some(ty_node) => extract_type_ids(ty_node)?,
        None => HashSet::new(),
    };

    Ok((key, ty_opt, type_names, span, typed_item))
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
    if *type_node.kind() != AstKind::Type {
        return Err(ParserInternalError::new(format!(
            "Expected Type node, found: {:?}",
            type_node.kind()
        )));
    }

    let mut ids = HashSet::new();

    for child in type_node.children() {
        if *child.kind() != AstKind::PrimitiveType {
            return Err(ParserInternalError::new(format!(
                "Expected PrimitiveType inside Type node, found: {:?}",
                child.kind()
            )));
        }

        ids.insert(child.try_ident()?);
    }

    Ok(ids)
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
    let entry = type_sources.entry(key).or_insert_with(|| (HashSet::new(), Vec::new()));
    entry.0.extend(type_names);
    entry.1.push(span.clone()); // No clone
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
    new_item: Box<AstNode>,
    mut new_type_opt: Option<Box<AstNode>>, // On prend ownership ici !
) -> bool {
    match merged_items.get_mut(&key) {
        Some(existing_item) => {
            let existing_children = existing_item.children_mut();

            if existing_children.len() > 1 {
                if let Some(mut new_type) = new_type_opt.take() { // on *prend* new_type, ownership
                    let existing_type = &mut existing_children[1];
                    let before = existing_type.children().len();

                    existing_type
                        .children_mut()
                        .extend(new_type.children_mut().drain(..)); // move sans clone

                    let after = existing_type.children().len();
                    return after > before;
                }
            }

            if existing_children.len() == 1 {
                if let Some(new_type) = new_type_opt.take() {
                    existing_children.push(new_type); // move direct sans clone
                    return true;
                }
            }

            false
        }
        None => {
            merged_items.insert(key, new_item); // move direct aussi
            true
        }
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
            let ty_name: String = ast.try_resolve(*ty)?.to_string();

            // Resolve and collect the interned names for each variant type ID
            let type_names = types_set
                .iter()
                .map(|id| ast.try_resolve(*id).map(|s| s.to_string()))
                .collect::<Result<_, _>>()?;

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
