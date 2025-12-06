//! Module responsible for normalizing and merging type_checker declarations in the AST.
//!
//! This module provides functions to process and clean up the abstract syntax arena (AST)
//! related to type_checker declarations. Its main purpose is to:
//!
//! - Detect and warn about implicit "either" type_checker declarations, which may indicate ambiguous or
//!   overlapping type_checker definitions.
//! - Merge duplicate type_checker declarations sharing the same primitive type_checker key to simplify and
//!   consolidate the AST.
//!
//! # Key Functions
//!
//! - [`normalize_type_def`]: The primary function that coordinates normalization by reporting
//!   warnings and merging duplicates.
//! - [`report_implicit_either_type_warning`]: Scans type_checker declarations to find implicit either types
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
//! use crate::aiplan4rust::syntax::ast::Ast;
//! use crate::aiplan4rust::diagnostic::DiagnosticManager;
//! use crate::your_module::normalize_type_def;  // replace with actual module path
//!
//! // Assume `ast` is parsed and initially normalized (e.g., typed lists normalized).
//! let mut ast: Ast = parse_source_code(source)?;
//! let mut diagnostics = DiagnosticManager::new();
//!
//! // Perform type definition normalization with diagnostics collection.
//! let changed = normalize_type_def(&mut ast, &mut diagnostics)?;
//!
//! if changed {
//!     println!("Type declarations merged successfully.");
//! }
//!
//! // Inspect diagnostics for warnings or errors generated during normalization.
//! for diagnostic in diagnostics.diagnostics() {
//!     println!("Diagnostic: {}", diagnostic);
//! }
//! ```

use std::collections::HashMap;
use std::collections::HashSet;

use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Provider};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::normalization::passes::NormalizationPassError;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::tree::NodeId;
use crate::aiplan4rust::syntax::tree::SyntaxNode;
use crate::aiplan4rust::syntax::Span;

/// Normalizes type_checker declarations in the AST by merging all `TypedItem` nodes
/// that share the same `PrimitiveType` key.
///
/// This function assumes that:
/// - The AST is already **valid** (i.e., produced by a correct parser and validated).
/// - All `TypedList` nodes have already been **normalized** (via the `normalize_typed_list` pass),
///   meaning that each `TypedItem` contains exactly one element and an optional type_checker.
///
/// It performs the following operations:
/// 1. Locates the `TypedList` syntax inside the `TypesDef` syntax of the AST.
/// 2. Extracts all `TypedItem` nodes from the `TypedList`.
/// 3. For each `TypedItem`, extracts:
///     - Its key (`PrimitiveType` string),
///     - Its optional type_checker annotation (`Type` syntax),
///     - The set of contained type_checker names,
///     - Its source span (for diagnostics).
/// 4. Tracks where each type_checker was declared (to identify duplicates).
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
/// - `Ok(true)` if the AST was modified (i.e., at least one type_checker was merged).
/// - `Ok(false)` if no changes were needed.
/// - `Err(NormalizationPassError)` if validation or extraction of any type_checker fails.
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
///     println!("Merged type_checker declarations.");
/// }
/// ```
pub fn normalize_type_def(
    ast: &mut Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, NormalizationPassError> {
    // Retrieve the syntax ID of the TypesDef syntax in the AST
    let types_def_id = match ast.find_node_id_of_kind(AstKind::TypesDef) {
        Some(id) => id,
        None => return Ok(false),
    };

    // Report warnings for implicit either type_checker declarations in the TypesDef syntax
    report_implicit_either_type_warning(types_def_id, ast, diagnostic_manager)?;

    // Merge duplicate TypedItem declarations sharing the same PrimitiveType key
    let modified = merge_duplicate_type_declarations(types_def_id, ast)?;

    Ok(modified)
}

/// Scans the type_checker declarations under the given syntax and reports warnings
/// for implicit 'either' type_checker declarations detected.
///
/// # Arguments
/// * `types_def_id` - The NodeId of the type_checker definitions syntax in the AST.
/// * `ast` - Reference to the AST arena for accessing nodes and their data.
/// * `diagnostic_manager` - Mutable reference to the diagnostic manager to record warnings.
///
/// # Returns
/// * `Ok(())` on success.
/// * `Err(NormalizationPassError)` if any AST access fails.
///
/// # Behavior
/// Iterates over all type_checker declarations within the `types_def_id` syntax.
/// For each type_checker, it collects the primitive type_checker identifier and the set of
/// super type_checker identifiers. If a primitive type_checker is encountered more than once,
/// it triggers a warning for implicit 'either' type_checker declaration at that type_checker's span.
fn report_implicit_either_type_warning(
    types_def_id: NodeId,
    ast: &Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), NormalizationPassError> {
    let seen = collect_implicit_either_type_declarations(types_def_id, ast)?;
    emit_implicit_either_type_warnings(seen, ast, diagnostic_manager)
}


/// Collects primitive types along with their super types and any duplicate declarations.
///
/// This function traverses a list of type declarations in the AST and identifies
/// primitive types that are declared multiple times with potentially different sets
/// of super types. It returns a map where each key is a type identifier (`Ident`)
/// and the value is a tuple containing:
/// - the set of super types from the first declaration,
/// - the source code span of the first declaration,
/// - and a list of duplicate declarations, each with their own set of super types and span.
///
/// This data can later be used to emit diagnostics indicating implicit `(either ...)`
/// type interpretations.
///
/// # Arguments
/// * `types_def_id` - The ID of the AST node representing the top-level type definitions.
/// * `ast` - The AST structure used to retrieve syntax nodes and identifiers.
///
/// # Returns
/// * `Ok(HashMap<...>)` containing all primitive types and their duplicate declarations.
/// * `Err(NormalizationPassError)` if any AST traversal or identifier extraction fails.
fn collect_implicit_either_type_declarations(
    types_def_id: NodeId,
    ast: &Ast,
) -> Result<HashMap<Ident, (HashSet<Ident>, Span, Vec<(HashSet<Ident>, Span)>)>, NormalizationPassError> {
    let syntax_tree = ast.syntax_tree();
    let typed_def_node = syntax_tree.try_node(types_def_id)?;
    let typed_list_id = typed_def_node.try_child(0)?;
    let typed_list = syntax_tree.try_node(typed_list_id)?;

    let mut seen: HashMap<Ident, (HashSet<Ident>, Span, Vec<(HashSet<Ident>, Span)>)> = HashMap::new();

    for typed_item_id in typed_list.children() {
        let type_item = syntax_tree.try_node(*typed_item_id)?;
        let primitive_type_id = type_item.try_child(0)?;
        let primitive_type = syntax_tree.try_node(primitive_type_id)?;
        let primitive_type_ident = primitive_type.try_ident()?;
        let primitive_type_span = primitive_type.span();

        let super_type_idents = match type_item.get_child(1) {
            Some(ty_id) => {
                let ty = syntax_tree.try_node(ty_id)?;
                let mut super_type_idents = HashSet::new();
                for super_type_id in ty.children() {
                    let super_type = syntax_tree.try_node(*super_type_id)?;
                    let super_type_ident = super_type.try_ident()?;
                    super_type_idents.insert(super_type_ident);
                }
                super_type_idents
            }
            None => HashSet::new(),
        };

        if let Some((_, _, duplicates)) = seen.get_mut(&primitive_type_ident) {
            duplicates.push((super_type_idents, primitive_type_span.clone()));
        } else {
            seen.insert(
                primitive_type_ident,
                (super_type_idents, primitive_type_span.clone(), Vec::new()),
            );
        }
    }

    Ok(seen)
}

/// Emits diagnostics for types with multiple conflicting declarations implicitly
/// interpreted as `(either ...)` types.
///
/// This function processes a map of type identifiers that have been declared more than once,
/// along with their associated super types and source code spans. For each type with duplicate
/// declarations, it generates a warning diagnostic indicating that the type was implicitly
/// treated as an `(either ...)` declaration due to the presence of multiple super type sets.
///
/// The diagnostic includes:
/// - the name of the conflicting type,
/// - the list of super types found in the duplicate declarations,
/// - the source code spans where these duplicate declarations occurred,
/// - and the span of the first declaration.
///
/// # Arguments
/// * `seen` - A map of type identifiers to their first declaration (with super types and span)
///   and a list of duplicate declarations (also with super types and spans).
/// * `ast` - The AST used to resolve identifiers and source information.
/// * `diagnostic_manager` - The manager used to emit diagnostics.
///
/// # Returns
/// * `Ok(())` if all diagnostics were emitted successfully.
/// * `Err(NormalizationPassError)` if any issue occurs during diagnostic creation.
fn emit_implicit_either_type_warnings(
    seen: HashMap<Ident, (HashSet<Ident>, Span, Vec<(HashSet<Ident>, Span)>)>,
    ast: &Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), NormalizationPassError> {
    for (ident, (_, first_span, duplicates)) in seen {
        if !duplicates.is_empty() {
            let mut duplicate_types = Vec::new();
            let mut duplicate_spans = Vec::new();

            for (supertypes, span) in duplicates {
                for st in supertypes {
                    duplicate_types.push(st);
                    duplicate_spans.push(span.clone());
                }
            }

            let warning = Diagnostic::warning_implicit_either_type_declaration(
                ident,
                duplicate_types,
                duplicate_spans,
                Provider::Normalizer, // Mark the simplify as the source of this warning
                ast.source_id(),      // Source file name where the warning originates
                first_span.clone(),   // Location span in the source code for the warning
            );

            diagnostic_manager.add_diagnostic(warning);
        }
    }

    Ok(())
}

/// Merges duplicate type_checker declarations in the AST by combining their supertype children.
///
/// This normalization pass traverses a list of type_checker declarations found under the provided
/// `types_def_id` node in the AST. If multiple type_checker declarations use the same primitive identifier
/// (e.g., multiple `(type_checker robot ...)` blocks with the same name), their child nodes are merged
/// into a single consolidated declaration.
///
/// # Parameters
///
/// - `types_def_id`: The [`NodeId`] of the parent syntax node containing the list of type_checker declarations.
/// - `ast`: A mutable reference to the [`Ast`] arena representing the syntax tree.
///
/// # Returns
///
/// - `Ok(true)` if any duplicate declarations were found and merged.
/// - `Ok(false)` if no duplicates were found and no changes were made.
/// - `Err(NormalizationPassError)` if traversal or AST manipulation fails.
///
/// # Behavior
///
/// - Scans the children of `types_def_id`, which is expected to contain a `(types ...)` list.
/// - For each type_checker declaration:
///   - Extracts the primitive type_checker name (e.g., `robot`, `vehicle`, etc.).
///   - Checks for prior declarations with the same type_checker name.
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
/// # use aiplan4rust::normalization::NormalizationPassError;
/// # fn example() -> Result<(), NormalizationPassError> {
/// let mut ast = AstArena::new();
/// let types_def_id = NodeId::new(1); // ID pointing to the `(types ...)` declaration
///
/// let changed = merge_duplicate_type_declarations(types_def_id, &mut ast)?;
///
/// if changed {
///     println!(" Duplicate type_checker declarations were successfully merged.");
/// } else {
///     println!("️ No duplicate type_checker declarations found.");
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// This function may return a [`NormalizationPassError`] if:
///
/// - The `types_def_id` does not point to a valid node or one without children.
/// - Any referenced child node is malformed or of unexpected kind.
/// - The AST arena encounters internal allocation or lookup issues.
///
/// # Assumptions
///
/// - The `types_def_id` node must have a `List` kind and represent a `(types ...)` clause.
/// - All type_checker declarations are immediate children of this node.
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
) -> Result<bool, NormalizationPassError> {
    // Get mutable access to the syntax tree holding all AST nodes
    let syntax_tree = ast.syntax_tree_mut();

    // Retrieve the syntax containing the types definitions
    let typed_def_node = syntax_tree.try_node(types_def_id)?;

    // The first child of this syntax is assumed to be the list syntax holding all type_checker declarations
    let typed_list_id = typed_def_node.try_child(0)?;
    let typed_list = syntax_tree.try_node_mut(typed_list_id)?;

    // Track whether any modifications happen (merges performed)
    let mut modified = false;

    // Map primitive type_checker identifiers to their first encountered declaration NodeId
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

        // Get the current type_checker declaration syntax
        let type_item = syntax_tree.try_node(typed_item_id)?;

        // Extract the primitive type_checker syntax and its identifier
        let primitive_type_id = type_item.try_child(0)?;
        let primitive_type = syntax_tree.try_node(primitive_type_id)?;
        let primitive_type_ident = primitive_type.try_ident()?;

        if let Some(&existing_item_id) = seen.get(&primitive_type_ident) {
            // Duplicate found: merge this declaration's children into the existing one

            // Get the existing declaration syntax and its "super type_checker" children syntax
            let existing_item = syntax_tree.try_node(existing_item_id)?;
            let existing_super_type_id = existing_item.try_child(1)?;

            // Get the current duplicate's "super type_checker" children syntax
            let current_super_type_id = type_item.try_child(1)?;

            // Retrieve nodes representing the children lists
            let existing_super_type = syntax_tree.try_node(existing_super_type_id)?;
            let current_super_type = syntax_tree.try_node(current_super_type_id)?;

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
            let existing_super_type_mut = syntax_tree.try_node_mut(existing_super_type_id)?;
            existing_super_type_mut.set_children(merged_children);

            // Mark this duplicate declaration for removal later
            duplicates_to_remove.insert(typed_item_id);

            // Remember that we modified the AST
            modified = true;
        } else {
            // First time seeing this primitive type_checker; record its declaration syntax
            seen.insert(primitive_type_ident, typed_item_id);
        }
    }

    // After processing all declarations, remove all duplicates in one operation
    if modified {
        let typed_list_mut = syntax_tree.try_node_mut(typed_list_id)?;

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
