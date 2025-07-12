//! Module responsible for normalizing and merging type declarations in the AST.
//!
//! This module provides functions to process and clean up the abstract syntax tree (AST)
//! related to type declarations. Its main purpose is to:
//!
//! - Detect and warn about implicit "either" type declarations, which may indicate ambiguous or overlapping type definitions.
//! - Merge duplicate type declarations sharing the same primitive type key to simplify and consolidate the AST.
//!
//! # Key Functions
//!
//! - [`normalize_type_def`]: The primary function that coordinates normalization by reporting warnings and merging duplicates.
//! - [`report_implicit_either_type_warning`]: Scans type declarations to find implicit either types and generates warnings.
//! - [`merge_duplicate_type_declarations`]: Merges `TypedItem` nodes with identical keys, updating the AST accordingly.
//!
//! # Usage Notes
//!
//! This module assumes that the AST has been previously validated and that the `TypedList` nodes
//! have been normalized (typically via the `normalize_typed_list` pass). Running these functions
//! on unnormalized ASTs can result in unexpected behavior or errors.
//!
//! # Examples
//!
//! ```rust,ignore
//! use crate::ast::AstArena;
//! use crate::diagnostics::DiagnosticManager;
//!
//! let mut ast = parse_source_code(source)?;
//! normalize_typed_list(&mut ast)?; // prerequisite normalization
//! let mut diagnostics = DiagnosticManager::new();
//! let changed = normalize_type_def(&mut ast, &mut diagnostics)?;
//! if changed {
//!     println!("Type declarations merged successfully.");
//! }
//! ```

use std::collections::HashMap;
use std::collections::HashSet;

use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::tree::{NodeId, ArenaNode};

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
/// Returns an error if the AST structure does not conform to expectations —
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
    // Retrieve the node ID of the TypesDef node in the AST
    let types_def_id = match ast.find_node_id_of_kind(AstKind::TypesDef) {
        Some(id) => id,
        None => return Ok(false),
    };

    // Report warnings for implicit either type declarations in the TypesDef node
    report_implicit_either_type_warning(types_def_id, ast, diagnostic_manager)?;

    // Merge duplicate TypedItem declarations sharing the same PrimitiveType key
    let modified = merge_duplicate_type_declarations(types_def_id, ast)?;

    Ok(modified)
}


/// Scans the type declarations under the given node and reports warnings
/// for implicit 'either' type declarations detected.
///
/// # Arguments
/// * `types_def_id` - The NodeId of the type definitions node in the AST.
/// * `ast` - Reference to the AST arena for accessing nodes and their data.
/// * `diagnostic_manager` - Mutable reference to the diagnostic manager to record warnings.
///
/// # Returns
/// * `Ok(())` on success.
/// * `Err(ParserInternalError)` if any AST access fails.
///
/// # Behavior
/// Iterates over all type declarations within the `types_def_id` node.
/// For each type, it collects the primitive type identifier and the set of
/// super type identifiers. If a primitive type is encountered more than once,
/// it triggers a warning for implicit 'either' type declaration at that type's span.
fn report_implicit_either_type_warning(
    types_def_id: NodeId,
    ast: &Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), ParserInternalError> {
    // Get immutable access to the arena containing the AST nodes
    let arena = ast.arena();

    // Retrieve the node representing the entire type definitions
    let typed_def_node = arena.try_node(types_def_id)?;

    // Get the first child which holds the list of typed declarations
    let typed_list_id = typed_def_node.try_child(0)?;
    let typed_list = arena.try_node(typed_list_id)?;

    // Map to keep track of seen primitive type identifiers and their super types
    let mut seen = HashMap::new();

    // Iterate over all type declaration nodes
    for typed_item_id in typed_list.children() {
        let type_item = arena.try_node(*typed_item_id)?;

        // Extract the primitive type identifier node and get its Ident
        let primitive_type_id = type_item.try_child(0)?;
        let primitive_type = arena.try_node(primitive_type_id)?;
        let primitive_type_ident = primitive_type.try_ident()?;


        // Extract the node containing super types of this primitive type if they exist
        let super_type_idents = match type_item.get_child(1) {
            Some(ty_id) => {
                let ty = arena.try_node(ty_id)?;
                // Collect all super type identifiers into a set
                let mut super_type_idents = HashSet::new();
                for super_type_id in ty.children() {
                    let super_type = arena.try_node(*super_type_id)?;
                    let super_type_ident = super_type.try_ident()?;
                    super_type_idents.insert(super_type_ident);
                }
                super_type_idents
            }
            None => HashSet::new(),
        };

        // Check if we've already seen this primitive type before
        if seen.contains_key(&primitive_type_ident) {
            // Generate a warning diagnostic for implicit either type declaration
            let warning = new_implicit_either_type_warning(primitive_type_ident, primitive_type.span(), ast)?;
            // Add the warning to the diagnostic manager
            diagnostic_manager.add_diagnostic(warning);
        } else {
            // Record this primitive type and its super types as seen
            seen.insert(primitive_type_ident, super_type_idents);
        }
    }

    // Successfully processed all type declarations without errors
    Ok(())
}


/// Creates a warning diagnostic for an implicit 'either' type declaration.
///
/// # Arguments
/// * `type_ident` - The identifier of the type for which the warning is generated.
/// * `span` - The source code span where the implicit type declaration occurs.
/// * `ast` - Reference to the AST arena, used to resolve the identifier to its string name.
///
/// # Returns
/// * `Ok(Diagnostic)` containing the warning information if successful.
/// * `Err(ParserInternalError)` if resolving the identifier fails.
///
/// # Purpose
/// This function generates a diagnostic warning indicating that an 'either' type
/// was implicitly declared, which may require attention from the user.
fn new_implicit_either_type_warning(
    type_ident: Ident,
    span: &Span,
    ast: &Ast,
) -> Result<Diagnostic, ParserInternalError> {

    // Resolve the string name of the type identifier using the AST's interner
    let type_name = ast.interner().try_resolve(type_ident)?;

    // Build the diagnostic object with relevant information
    let diagnostic = Diagnostic::new(
        DiagnosticKind::ImplicitEitherTypeDeclarationWarning {
            ty: type_name.to_string(),
        },
        Provider::Normalizer,           // Mark the normalizer as the source of this warning
        ast.source_name().to_string(),  // Source file name where the warning originates
        span.clone(),                   // Location span in the source code for the warning
    );

    // Return the constructed diagnostic wrapped in Ok
    Ok(diagnostic)
}


/// Merges duplicate type declarations in the AST by combining their children.
///
/// This function scans through a list of type declarations under a given node (`types_def_id`)
/// in the AST arena. If multiple declarations share the same primitive type identifier,
/// their children nodes (super types) are merged into a single declaration,
/// removing duplicates and preserving the order where possible.
///
/// # Parameters
/// - `types_def_id`: The `NodeId` of the parent node containing the type declarations list.
/// - `ast`: A mutable reference to the `AstArena` containing the AST nodes.
///
/// # Returns
/// - `Ok(true)` if any duplicate declarations were merged and the AST was modified.
/// - `Ok(false)` if no duplicates were found and no changes were made.
/// - `Err(ParserInternalError)` if accessing nodes or children fails during traversal.
///
/// # Behavior
/// For each type declaration, the function extracts the primitive type identifier.
/// If a previous declaration with the same identifier exists, it merges the children of the
/// current declaration into the existing one, removing any duplicate children.
/// The current duplicate declaration node is then removed from its parent's children list.
///
/// # Example
/// ```rust,no_run
/// # // Assume existence of ast arena setup and types_def_id.
/// # let mut ast = AstArena::new();
/// # let types_def_id = NodeId::new(0);
/// let modified = merge_duplicate_type_declarations(types_def_id, &mut ast)?;
/// if modified {
///     println!("Duplicate type declarations merged successfully.");
/// } else {
///     println!("No duplicates found.");
/// }
/// # Ok::<(), ParserInternalError>(())
/// ```
///
/// # Errors
/// This function returns an error if any of the node retrievals or child accesses fail,
/// which usually indicates an inconsistent or malformed AST.
///
/// # Notes
/// - This function assumes the first child of the node with `types_def_id` is a list node
///   containing the individual type declarations.
/// - The function operates in-place, mutating the provided AST arena.
///
/// # See Also
/// Related functions that manipulate the AST or type declarations.
///
/// ```
pub fn merge_duplicate_type_declarations(
    types_def_id: NodeId,
    ast: &mut Ast,
) -> Result<bool, ParserInternalError> {
    // Get mutable access to the arena holding all AST nodes
    let arena = ast.arena_mut();

    // Retrieve the node containing the types definitions
    let typed_def_node = arena.try_node(types_def_id)?;

    // The first child of this node is assumed to be the list node holding all type declarations
    let typed_list_id = typed_def_node.try_child(0)?;
    let typed_list = arena.try_node_mut(typed_list_id)?;

    // Track whether any modifications happen (merges performed)
    let mut modified = false;

    // Map primitive type identifiers to their first encountered declaration NodeId
    let mut seen: HashMap<Ident, NodeId> = HashMap::new();

    // Collect IDs of duplicate declarations that need to be removed later
    let mut duplicates_to_remove: HashSet<NodeId> = HashSet::new();

    // Create a stable snapshot of the children list to iterate over
    let children_ids = typed_list.children().to_vec();

    for &typed_item_id in &children_ids {
        // Skip any node already marked as duplicate to avoid redundant processing
        if duplicates_to_remove.contains(&typed_item_id) {
            continue;
        }

        // Get the current type declaration node
        let type_item = arena.try_node(typed_item_id)?;

        // Extract the primitive type node and its identifier
        let primitive_type_id = type_item.try_child(0)?;
        let primitive_type = arena.try_node(primitive_type_id)?;
        let primitive_type_ident = primitive_type.try_ident()?;

        if let Some(&existing_item_id) = seen.get(&primitive_type_ident) {
            // Duplicate found: merge this declaration's children into the existing one

            // Get the existing declaration node and its "super type" children node
            let existing_item = arena.try_node(existing_item_id)?;
            let existing_super_type_id = existing_item.try_child(1)?;

            // Get the current duplicate's "super type" children node
            let current_super_type_id = type_item.try_child(1)?;

            // Retrieve nodes representing the children lists
            let existing_super_type = arena.try_node(existing_super_type_id)?;
            let current_super_type = arena.try_node(current_super_type_id)?;

            // Start merged list with existing children
            let mut merged_children = existing_super_type.children().to_vec();

            // Track which children have been seen for deduplication
            let mut seen_children: HashSet<NodeId> = merged_children.iter().copied().collect();

            // Append new children from the current declaration that aren't duplicates
            for &child_id in current_super_type.children() {
                if !seen_children.contains(&child_id) {
                    merged_children.push(child_id);
                    seen_children.insert(child_id);
                }
            }

            // Update the existing declaration's children to the merged list
            let mut existing_super_type_mut = arena.try_node_mut(existing_super_type_id)?;
            existing_super_type_mut.set_children(merged_children);

            // Mark this duplicate declaration for removal later
            duplicates_to_remove.insert(typed_item_id);

            // Remember that we modified the AST
            modified = true;
        } else {
            // First time seeing this primitive type; record its declaration node
            seen.insert(primitive_type_ident, typed_item_id);
        }
    }

    // After processing all declarations, remove all duplicates in one operation
    if modified {
        let mut typed_list_mut = arena.try_node_mut(typed_list_id)?;

        // Filter out all nodes marked as duplicates from the children list
        typed_list_mut.set_children(
            typed_list_mut
                .children()
                .iter()
                .copied()
                .filter(|id| !duplicates_to_remove.contains(id))
                .collect(),
        );
    }

    // Return whether any merge was performed
    Ok(modified)
}
