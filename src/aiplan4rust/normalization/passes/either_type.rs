//! Module for normalization pass to detect and remove duplicate `PrimitiveType` children within `Type` nodes in the AST.
//!
//! This module provides functionality to:
//! - Traverse the abstract syntax tree (AST) and detect duplicate type identifiers inside `Type` nodes.
//! - Emit diagnostic warnings immediately upon detecting duplicates.
//! - Remove duplicate `PrimitiveType` children from `Type` nodes to normalize the AST.
//!
//! The main entry point is [`normalize_either_type`], which performs the detection (with diagnostics)
//! and the removal in two separate steps.
//!
//! # Overview
//!
//! In the domain-specific language AST, `Type` nodes can have children that represent primitive types.
//! It is invalid or redundant for a `Type` node to contain duplicate primitive type identifiers
//! (e.g., `(either t1 t1)`).
//!
//! This module ensures the AST is normalized by:
//! 1. Reporting duplicate identifiers with detailed diagnostics including source spans.
//! 2. Removing duplicate `PrimitiveType` children, keeping only the first occurrence.
//!
//! # Provided Functions
//!
//! - `normalize_either_type` — Main function that runs detection and removal steps.
//! - `report_either_type_duplicate_warnings` — Detects and reports duplicates via diagnostics.
//! - `new_duplicate_either_type_warning` — Helper to build diagnostic warnings for duplicates.
//! - `remove_either_type_duplicates` — Removes duplicate `PrimitiveType` children in-place.
//!
//! # Errors
//!
//! These functions may return `ParserInternalError` if any AST node access or mutation fails,
//! or if identifier resolution for diagnostics encounters errors.
//!
//! # Usage Example
//!
//! ```rust
//! use crate::aiplan4rust::normalization::passes::either_type::normalize_either_type;
//! use crate::aiplan4rust::syntax::ast::AstArena;
//! use crate::aiplan4rust::diagnostic::DiagnosticManager;
//!
//! let mut ast = AstArena::new(...);
//! let mut diagnostic_manager = DiagnosticManager::new();
//!
//! match normalize_either_type(&mut ast, &mut diagnostic_manager) {
//!     Ok(modified) if modified => println!("AST normalized, duplicates removed."),
//!     Ok(_) => println!("No duplicates found; AST unchanged."),
//!     Err(e) => eprintln!("Normalization error: {:?}", e),
//! }
//! ```
//!
//! # Imports
//!
//! The module depends on:
//! - Standard collections (`HashSet`) for duplicate detection.
//! - AST structures and kinds for node manipulation.
//! - Diagnostic management for warnings.
//! - Error types for parser internal errors.
//!
//! See individual function docs for detailed behavior and examples.

use std::collections::HashSet;
use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::{AstArenaNode, AstArena, AstContent};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::tree::TreeArena;

/// Normalizes all `Type` nodes in the AST by detecting and removing duplicate `PrimitiveType` children.
///
/// This function performs two main steps:
/// 1. It traverses the AST to detect and immediately report any duplicate `PrimitiveType` identifiers
///    found within `Type` nodes via the provided `diagnostic_manager`.
/// 2. It then removes those duplicate `PrimitiveType` nodes from the AST, modifying it in place.
///
/// This helps ensure that `Type` nodes do not contain redundant type specifications (e.g., `(either t1 t1)`).
///
/// # Parameters
///
/// - `ast`: A mutable reference to the `AstArena` representing the abstract syntax tree to normalize.
/// - `diagnostic_manager`: A mutable reference to the `DiagnosticManager` used to collect and report
///   warnings about duplicate types.
///
/// # Returns
///
/// - `Ok(true)` if any duplicates were found and removed (i.e., the AST was modified).
/// - `Ok(false)` if no duplicates were found and no modifications were needed.
/// - `Err(ParserInternalError)` if an error occurred while traversing or modifying the AST.
///
/// # Behavior
///
/// - Duplicate detection reports warnings immediately, but does not modify the AST.
/// - Duplicate removal happens after reporting and modifies the AST by removing redundant nodes.
/// - The function relies on correct AST and arena implementations to safely access and mutate nodes.
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::normalization::passes::either_type::normalize_either_type;
/// use crate::aiplan4rust::syntax::ast::AstArena;
/// use crate::aiplan4rust::diagnostic::DiagnosticManager;
/// use crate::aiplan4rust::parser::ParserInternalError;
///
/// let mut ast = AstArena::new(...);
/// let mut diagnostic_manager = DiagnosticManager::new();
///
/// match normalize_either_type(&mut ast, &mut diagnostic_manager) {
///     Ok(modified) => {
///         if modified {
///             println!("AST was normalized and duplicates removed.");
///         } else {
///             println!("No duplicates found; AST unchanged.");
///         }
///     }
///     Err(e) => eprintln!("Error during normalization: {:?}", e),
/// }
/// ```
///
pub fn normalize_either_type(
    ast: &mut AstArena,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let arena = ast.arena();
    // Step 1: Detect and report duplicate type warnings without modifying the AST
    report_either_type_duplicate_warnings(arena, ast, diagnostic_manager)?;

    // Step 2: Mutably borrow arena to remove duplicates and track if modifications were made
    let arena = ast.arena_mut();
    let modified = remove_either_type_duplicates(arena)?;

    Ok(modified)
}

/// Traverses the AST to detect and report duplicate identifiers within 'Type' nodes.
///
/// This function performs a preorder traversal over the AST nodes contained in the provided `arena`.
/// For each node of kind `Type`, it inspects its immediate children to identify duplicates among
/// `PrimitiveType` children, specifically by checking their identifier (`Ident`) content. If duplicates
/// are found, it reports these as warnings through the provided `diagnostic_manager`.
///
/// The function uses `ast` to resolve identifiers to their string representations for meaningful diagnostics.
///
/// # Parameters
///
/// - `arena`: Reference to the tree arena containing `AstArenaNode` nodes. This is the structure
///   holding the AST nodes to traverse.
/// - `ast`: Reference to the AST arena, used to resolve `Ident` to human-readable strings.
/// - `diagnostic_manager`: Mutable reference to the diagnostic manager, used to emit warnings about duplicates.
///
/// # Returns
///
/// - `Ok(())` if traversal and reporting complete successfully.
/// - `Err(ParserInternalError)` if any node or identifier resolution fails during traversal.
///
/// # Behavior
///
/// - Traverses the AST in preorder to ensure parent nodes are processed before children.
/// - For each `Type` node, collects identifiers of its `PrimitiveType` children.
/// - Detects duplicates among these identifiers and collects them.
/// - Calls `report_duplicate_either_type_warning_bis` to report warnings for duplicates found.
///
/// # Example
///
/// ```ignore
/// let result = report_either_type_duplicate_warnings(&arena, &ast, &mut diagnostic_manager);
/// if let Err(e) = result {
///     eprintln!("Error during duplicate detection: {:?}", e);
/// }
/// ```
///
/// # Notes
///
/// - This function does **not** modify the AST; it only detects and reports duplicates.
/// - It relies on correct implementations of `try_node` and `try_resolve` for safe access.
///
/// # See Also
///
/// - `report_duplicate_either_type_warning_bis` – helper function that actually formats and sends diagnostics.
fn report_either_type_duplicate_warnings(
    arena: &TreeArena<AstArenaNode>,
    ast: &AstArena,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), ParserInternalError> {
    // Retrieve the source name from the AST, used for diagnostics reporting
    let source_name = ast.source_name();

    // Traverse all nodes in the AST in preorder (parent before children)
    for node in arena.preorder() {
        // Skip nodes that are not of kind Type, since duplicates only matter there
        if node.kind() != AstKind::Type {
            continue;
        }

        // HashSet to track which identifiers have already been seen in this Type node
        let mut seen = HashSet::new();
        // Vector to collect identifiers detected as duplicates
        let mut duplicates = Vec::new();

        // Iterate over immediate children of the current Type node
        for &child_id in node.children() {
            // Get an immutable reference to the child node
            let child = arena.try_node(child_id)?;

            // Check if the child node is a PrimitiveType (the relevant node kind for IDs)
            if child.kind() == AstKind::PrimitiveType {
                // Extract the identifier content from the node
                if let AstContent::Ident(id) = child.content() {
                    // If this identifier was already seen, record it as a duplicate
                    if !seen.insert(*id) {
                        duplicates.push(*id);
                    }
                }
            }
        }

        // If any duplicates were found, create and add a diagnostic warning
        if !duplicates.is_empty() {
            // Construct a diagnostic warning for these duplicates
            let warning = new_duplicate_either_type_warning(
                duplicates,
                ast,
                source_name,
                &node.span(),
            )?;
            // Add the diagnostic to the diagnostic manager for reporting
            diagnostic_manager.add_diagnostic(warning);
        }
    }

    // Indicate successful completion without errors
    Ok(())
}
/// Creates a diagnostic warning for duplicate identifiers found within a 'Type' node.
///
/// Given a vector of duplicate identifier (`Ident`) values, this function resolves each identifier
/// to its string representation using the provided `ast`. It then constructs a `Diagnostic`
/// object containing the duplicates and associated source information.
///
/// # Parameters
///
/// - `duplicate_ids`: Vector of `Ident` representing the duplicate identifiers detected.
/// - `ast`: Reference to the AST arena for resolving identifiers.
/// - `source`: Source filename or identifier where the duplicates were found.
/// - `span`: The source code span indicating the location of the duplicate identifiers.
///
/// # Returns
///
/// - `Ok(Diagnostic)` containing the formatted warning ready to be emitted.
/// - `Err(ParserInternalError)` if any identifier resolution fails.
///
/// # Example
///
/// ```ignore
/// let diagnostic = create_duplicate_either_type_warning(duplicate_ids, &ast, source, &span)?;
/// diagnostic_manager.add_diagnostic(diagnostic);
/// ```
///
/// # Notes
///
/// - This function only creates and returns the diagnostic; it does **not** report it.
/// - The caller is responsible for submitting the diagnostic to the diagnostic manager.
fn new_duplicate_either_type_warning(
    duplicate_ids: Vec<Ident>,
    ast: &AstArena,
    source: &str,
    span: &Span,
) -> Result<Diagnostic, ParserInternalError> {
    // Resolve identifiers to strings
    let duplicates: Vec<String> = duplicate_ids
        .into_iter()
        .map(|id| ast.try_resolve(id).map(str::to_string))
        .collect::<Result<_, _>>()?;

    // Build diagnostic warning
    let diagnostic = Diagnostic::new(
        DiagnosticKind::DuplicateEitherTypeWarning { duplicate_types: duplicates },
        Provider::Normalizer,
        source.to_string(),
        span.clone(),
    );

    Ok(diagnostic)
}

/// Removes duplicate `PrimitiveType` children within `Type` nodes in the AST.
///
/// This function traverses the AST starting from the root node in a depth-first manner,
/// visiting every node. For each node of kind `Type`, it inspects its immediate children
/// and removes duplicates among those children whose kind is `PrimitiveType` and which
/// share the same identifier (`Ident`). Only the first occurrence of each identifier is kept.
///
/// # Parameters
///
/// - `arena`: A mutable reference to the tree arena containing `AstArenaNode` nodes. This is
///   the data structure representing the AST.
///
/// # Returns
///
/// - `Ok(true)` if any duplicates were removed (i.e., the AST was modified).
/// - `Ok(false)` if no duplicates were found and the AST remains unchanged.
/// - `Err(ParserInternalError)` if an error occurs while accessing nodes in the arena.
///
/// # Behavior
///
/// - Traverses the AST iteratively using a stack to avoid recursion.
/// - Collects children IDs immutably before mutating the node to avoid borrowing conflicts.
/// - Uses a hash set to track seen identifiers and detect duplicates efficiently.
/// - Updates the children list of `Type` nodes to exclude duplicates.
///
/// # Example
///
/// ```ignore
/// let modified = remove_either_type_duplicates(&mut arena)?;
/// if modified {
///     println!("Duplicates removed from the AST.");
/// } else {
///     println!("No duplicates found.");
/// }
/// ```
///
/// # Errors
///
/// Returns an error if any node cannot be accessed or mutated properly during traversal.
///
/// # Notes
///
/// - This function only removes duplicate `PrimitiveType` children inside `Type` nodes.
/// - The ordering of children is preserved except duplicates are removed.
///
/// # See also
///
/// - `normalize_either_type` – calls this function as part of its normalization pipeline.
fn remove_either_type_duplicates(
    arena: &mut TreeArena<AstArenaNode>,
) -> Result<bool, ParserInternalError> {
    let mut modified = false;
    let mut stack = vec![arena.try_root_id()?];

    while let Some(node_id) = stack.pop() {
        // Obtain an immutable reference to the current node for reading
        let node = arena.try_node(node_id)?;
        // Clone the children IDs to avoid borrowing issues when mutating later
        let children_ids = node.children().to_vec();
        // Cache the node kind for quick checks
        let node_kind = node.kind();

        // Process only nodes of kind 'Type' to remove duplicate PrimitiveType children
        if node_kind == AstKind::Type {
            let mut seen = HashSet::new();  // Track seen identifiers to detect duplicates
            // Pre-allocate vector to hold filtered children with capacity = current children count
            let mut retained = Vec::with_capacity(children_ids.len());

            // Iterate over all children to filter out duplicate PrimitiveType identifiers
            for &child_id in &children_ids {
                let child = arena.try_node(child_id)?;
                match child.kind() {
                    AstKind::PrimitiveType => {
                        if let AstContent::Ident(id) = child.content() {
                            // Insert returns false if id was already present (duplicate)
                            if seen.insert(*id) {
                                retained.push(child_id); // Keep first occurrence
                            } else {
                                modified = true; // Mark that modification occurred by removing duplicate
                            }
                        } else {
                            // If PrimitiveType without Ident content, just keep it
                            retained.push(child_id);
                        }
                    }
                    // For other child kinds, keep them unchanged
                    _ => retained.push(child_id),
                }
            }

            // After reading and processing children, obtain mutable reference to update node
            let node_mut = arena.try_node_mut(node_id)?;
            node_mut.set_children(retained);
        }

        // Push all children onto the stack to continue depth-first traversal
        for child_id in children_ids {
            stack.push(child_id);
        }
    }

    // Return whether the AST was modified by removing duplicates
    Ok(modified)
}
