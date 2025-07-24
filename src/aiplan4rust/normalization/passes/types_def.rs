//! Module responsible for normalizing and merging type declarations in the AST.
//!
//! This module provides functions to process and clean up the abstract syntax arena (AST)
//! related to type declarations. Its main purpose is to:
//!
//! - Detect and warn about implicit "either" type declarations, which may indicate ambiguous or
//!   overlapping type definitions.
//! - Merge duplicate type declarations sharing the same primitive type key to simplify and
//!   consolidate the AST.
//!
//! # Key Functions
//!
//! - [`normalize_type_def`]: The primary function that coordinates normalization by reporting
//!   warnings and merging duplicates.
//! - [`report_implicit_either_type_warning`]: Scans type declarations to find implicit either types
//!   and generates warnings.
//! - [`merge_duplicate_type_declarations`]: Merges `TypedItem` nodes with identical keys, updating
//!   the AST accordingly.
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

use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::normalization::NormalizationError;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::tree::NodeId;
use crate::aiplan4rust::syntax::tree::SyntaxNode;
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
/// 1. Locates the `TypedList` syntax inside the `TypesDef` syntax of the AST.
/// 2. Extracts all `TypedItem` nodes from the `TypedList`.
/// 3. For each `TypedItem`, extracts:
///     - Its key (`PrimitiveType` string),
///     - Its optional type annotation (`Type` syntax),
///     - The set of contained type names,
///     - Its source span (for diagnostics).
/// 4. Tracks where each type was declared (to identify duplicates).
/// 5. Merges `TypedItem`s that share the same key by:
///     - Appending the contents of each `Type` syntax to the existing one if already present.
/// 6. Reports diagnostics if multiple declarations for the same key are detected.
/// 7. Replaces the children of the `TypedList` syntax with the merged list.
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
/// - `Err(NormalizationError)` if validation or extraction of any type fails.
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
) -> Result<bool, NormalizationError> {
    // Retrieve the syntax ID of the TypesDef syntax in the AST
    let types_def_id = match ast.find_node_id_of_kind(AstKind::TypesDef) {
        Some(id) => id,
        None => return Ok(false),
    };

    // Report warnings for implicit either type declarations in the TypesDef syntax
    report_implicit_either_type_warning(types_def_id, ast, diagnostic_manager)?;

    // Merge duplicate TypedItem declarations sharing the same PrimitiveType key
    let modified = merge_duplicate_type_declarations(types_def_id, ast)?;

    Ok(modified)
}

/// Scans the type declarations under the given syntax and reports warnings
/// for implicit 'either' type declarations detected.
///
/// # Arguments
/// * `types_def_id` - The NodeId of the type definitions syntax in the AST.
/// * `ast` - Reference to the AST arena for accessing nodes and their data.
/// * `diagnostic_manager` - Mutable reference to the diagnostic manager to record warnings.
///
/// # Returns
/// * `Ok(())` on success.
/// * `Err(NormalizationError)` if any AST access fails.
///
/// # Behavior
/// Iterates over all type declarations within the `types_def_id` syntax.
/// For each type, it collects the primitive type identifier and the set of
/// super type identifiers. If a primitive type is encountered more than once,
/// it triggers a warning for implicit 'either' type declaration at that type's span.
fn report_implicit_either_type_warning(
    types_def_id: NodeId,
    ast: &Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), NormalizationError> {
    // Get immutable access to the arena containing the AST nodes
    let arena = ast.arena();

    // Retrieve the syntax representing the entire type definitions
    let typed_def_node = arena.try_node(types_def_id)?;

    // Get the first child which holds the list of typed declarations
    let typed_list_id = typed_def_node.try_child(0)?;
    let typed_list = arena.try_node(typed_list_id)?;

    // Map to keep track of seen primitive type identifiers and their super types
    let mut seen = HashMap::new();

    // Iterate over all type declaration nodes
    for typed_item_id in typed_list.children() {
        let type_item = arena.try_node(*typed_item_id)?;

        // Extract the primitive type identifier syntax and get its Ident
        let primitive_type_id = type_item.try_child(0)?;
        let primitive_type = arena.try_node(primitive_type_id)?;
        let primitive_type_ident = primitive_type.try_ident()?;

        // Extract the syntax containing super types of this primitive type if they exist
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
            let warning =
                new_implicit_either_type_warning(primitive_type_ident, primitive_type.span(), ast)?;
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
/// * `Err(NormalizationError)` if resolving the identifier fails.
///
/// # Purpose
/// This function generates a diagnostic warning indicating that an 'either' type
/// was implicitly declared, which may require attention from the user.
fn new_implicit_either_type_warning(
    type_ident: Ident,
    span: &Span,
    ast: &Ast,
) -> Result<Diagnostic, NormalizationError> {
    // Resolve the string name of the type identifier using the AST's interner
    let type_name = ast.interner().try_resolve(type_ident)?;

    // Build the diagnostic object with relevant information
    let diagnostic = Diagnostic::new(
        DiagnosticKind::ImplicitEitherTypeDeclarationWarning {
            ty: type_name.to_string(),
        },
        Provider::Normalizer, // Mark the normalizer as the source of this warning
        ast.source_name().to_string(), // Source file name where the warning originates
        span.clone(),         // Location span in the source code for the warning
    );

    // Return the constructed diagnostic wrapped in Ok
    Ok(diagnostic)
}

/// Merges duplicate type declarations in the AST by combining their supertype children.
///
/// This normalization pass traverses a list of type declarations found under the provided
/// `types_def_id` node in the AST. If multiple type declarations use the same primitive identifier
/// (e.g., multiple `(type robot ...)` blocks with the same name), their child nodes are merged
/// into a single consolidated declaration.
///
/// # Parameters
///
/// - `types_def_id`: The [`NodeId`] of the parent syntax node containing the list of type declarations.
/// - `ast`: A mutable reference to the [`Ast`] arena representing the syntax tree.
///
/// # Returns
///
/// - `Ok(true)` if any duplicate declarations were found and merged.
/// - `Ok(false)` if no duplicates were found and no changes were made.
/// - `Err(NormalizationError)` if traversal or AST manipulation fails.
///
/// # Behavior
///
/// - Scans the children of `types_def_id`, which is expected to contain a `(types ...)` list.
/// - For each type declaration:
///   - Extracts the primitive type name (e.g., `robot`, `vehicle`, etc.).
///   - Checks for prior declarations with the same type name.
///   - If found, merges their supertype children, removing duplicates while preserving order.
///   - Removes the redundant declaration node from the parent's children list.
///
/// The function operates in-place, directly modifying the provided AST arena.
///
/// # Example
///
/// ```rust,no_run
/// # use aiplan4rust::syntax::tree::NodeId;
/// # use aiplan4rust::syntax::ast::{Ast, AstArena};
/// # use aiplan4rust::normalization::passes::merge_duplicate_type_declarations;
/// # use aiplan4rust::normalization::NormalizationError;
/// # fn example() -> Result<(), NormalizationError> {
/// let mut ast = AstArena::new();
/// let types_def_id = NodeId::new(1); // ID pointing to the `(types ...)` declaration
///
/// let changed = merge_duplicate_type_declarations(types_def_id, &mut ast)?;
///
/// if changed {
///     println!("✅ Duplicate type declarations were successfully merged.");
/// } else {
///     println!("ℹ️ No duplicate type declarations found.");
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// This function may return a [`NormalizationError`] if:
///
/// - The `types_def_id` does not point to a valid node or one without children.
/// - Any referenced child node is malformed or of unexpected kind.
/// - The AST arena encounters internal allocation or lookup issues.
///
/// # Assumptions
///
/// - The `types_def_id` node must have a `List` kind and represent a `(types ...)` clause.
/// - All type declarations are immediate children of this node.
/// - Duplicate detection is based on matching the primitive name (e.g., `robot`, `agent`).
///
/// # See Also
///
/// - [`normalize_type_def`] — Wrapper function that applies this merging as part of full normalization.
/// - [`Normalizer`] — Interface that orchestrates multiple normalization passes.
/// - [`Ast`] — The syntax tree structure being normalized.
/// - [`NodeId`] — Unique identifier for nodes in the AST arena.
///
/// # Related Passes
///
/// - [`normalize_require_def`] — Merges duplicate `:requirements`.
/// - [`normalize_typed_list`] — Expands and cleans up typed item declarations.
///
/// # Stability
///
/// This function is internal to normalization and may be refactored without notice.
/// It is not intended to be called outside the `passes` module.

pub fn merge_duplicate_type_declarations(
    types_def_id: NodeId,
    ast: &mut Ast,
) -> Result<bool, NormalizationError> {
    // Get mutable access to the arena holding all AST nodes
    let arena = ast.arena_mut();

    // Retrieve the syntax containing the types definitions
    let typed_def_node = arena.try_node(types_def_id)?;

    // The first child of this syntax is assumed to be the list syntax holding all type declarations
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
        // Skip any syntax already marked as duplicate to avoid redundant processing
        if duplicates_to_remove.contains(&typed_item_id) {
            continue;
        }

        // Get the current type declaration syntax
        let type_item = arena.try_node(typed_item_id)?;

        // Extract the primitive type syntax and its identifier
        let primitive_type_id = type_item.try_child(0)?;
        let primitive_type = arena.try_node(primitive_type_id)?;
        let primitive_type_ident = primitive_type.try_ident()?;

        if let Some(&existing_item_id) = seen.get(&primitive_type_ident) {
            // Duplicate found: merge this declaration's children into the existing one

            // Get the existing declaration syntax and its "super type" children syntax
            let existing_item = arena.try_node(existing_item_id)?;
            let existing_super_type_id = existing_item.try_child(1)?;

            // Get the current duplicate's "super type" children syntax
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
            let existing_super_type_mut = arena.try_node_mut(existing_super_type_id)?;
            existing_super_type_mut.set_children(merged_children);

            // Mark this duplicate declaration for removal later
            duplicates_to_remove.insert(typed_item_id);

            // Remember that we modified the AST
            modified = true;
        } else {
            // First time seeing this primitive type; record its declaration syntax
            seen.insert(primitive_type_ident, typed_item_id);
        }
    }

    // After processing all declarations, remove all duplicates in one operation
    if modified {
        let typed_list_mut = arena.try_node_mut(typed_list_id)?;

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
