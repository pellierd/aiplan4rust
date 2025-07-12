//! Module `require_def`
//!
//! This module provides functions to normalize `RequireDef` declarations within an AST,
//! specifically to detect and remove duplicate `Requirement` nodes.
//!
//! # Key Features
//!
//! - Locate the `RequireDef` node in the AST.
//! - Detect duplicate `Requirement` children.
//! - Emit diagnostic warnings for detected duplicates.
//! - Safely remove duplicate requirements from the `RequireDef` node.
//!
//! # Usage
//!
//! The primary function exposed is `normalize_require_def`, which performs a full pass on the `RequireDef` node:
//! - Finds the node in the AST.
//! - Reports duplicates through a diagnostic manager.
//! - Removes duplicate requirements.
//!
//! # Example
//!
//! ```rust
//! use crate::require_def::normalize_require_def;
//! use crate::diagnostics::DiagnosticManager;
//!
//! let mut ast = parse_source_code(source)?;
//! let mut diagnostics = DiagnosticManager::new();
//! let changed = normalize_require_def(&mut ast, &mut diagnostics)?;
//! if changed {
//!     println!("Duplicate requirements have been removed.");
//! }
//! ```
//!
//! # Notes
//!
//! This module assumes that the AST is valid and structurally sound.
//! Any inconsistency in node structure may cause errors or panics.
//!
//! The normalization pass is standalone and can be run independently at any point.

use std::collections::HashSet;
use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::arena::{NodeId, Arena};

/// Normalizes the requirement declarations by removing duplicates from the `RequireDef` node.
///
/// This function assumes the input AST is **valid** and structurally correct.
/// Specifically, if a `RequireDef` node is present, it should contain only well-formed `Requirement` nodes as children.
///
/// The normalization process does not traverse the entire AST. Instead, it directly locates the `RequireDef`
/// node (if any), which is expected to hold all `:requirement` declarations. It removes duplicate
/// requirements (i.e., those with identical keys) and emits a diagnostic warning for each duplicate found and removed.
///
/// This pass is **self-contained** and **stateless**—it does not depend on or affect any other normalization passes,
/// and can safely be run independently at any point, provided the AST is valid.
///
/// # Arguments
///
/// * `ast` - A mutable reference to the AST that may contain a `RequireDef` node.
/// * `diagnostic_manager` - A manager used to report warnings when duplicate requirements are detected.
///
/// # Returns
///
/// * `Ok(true)` if at least one duplicate requirement was removed.
/// * `Ok(false)` if no duplicates were found or if no `RequireDef` node exists.
/// * `Err(ParserInternalError)` if the AST structure is not as expected (e.g., invalid node kinds).
///
/// # Assumptions
///
/// * The AST must be valid and conform to the parser's grammar and invariants.
/// * The `RequireDef` node—if present—must contain only `Requirement` children.
/// * No other normalization passes are required before or after this one.
/// * This function is deterministic and has no side effects outside its scope.
///
/// # Panics
///
/// This function may panic if the `RequireDef` node contains unexpected children (e.g., non-`Requirement` nodes),
/// which indicates a violation of AST validity and a programming error.
///
/// # Example
///
/// ```rust
/// let mut ast = parse_source_code(source)?;
/// let mut diagnostics = DiagnosticManager::new();
/// let changed = normalize_require_def(&mut ast, &mut diagnostics)?;
/// if changed {
///     println!("Duplicate requirements were removed.");
/// }
/// ```
pub fn normalize_require_def(
    ast: &mut Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    // Early exit if no RequireDef node is found
    let require_def_id = match ast.find_node_id_of_kind(AstKind::RequireDef) {
        Some(id) => id,
        None => return Ok(false),
    };

    // Report duplicate requirement warnings (immutable borrow)
    report_duplicate_requirements_warnings(
        ast.arena(),
        require_def_id,
        ast.source_name(),
        diagnostic_manager,
    )?;

    // Remove duplicate requirements (mutable borrow)
    let modified = remove_requirement_duplicates(ast.arena_mut(), require_def_id)?;

    Ok(modified)
}

/// Scans the children of a `RequireDef` node in the AST to detect duplicate `:requirement` entries,
/// and reports a diagnostic warning if duplicates are found.
///
/// This function iterates over the children of the specified `RequireDef` node, collects duplicate
/// requirements, and generates a diagnostic warning which is added to the given diagnostic manager.
///
/// # Arguments
///
/// * `arena` - Reference to the AST arena containing the nodes.
/// * `require_def_id` - The `NodeId` of the `RequireDef` node to inspect.
/// * `source_name` - The name of the source file (used for context in diagnostics).
/// * `diagnostic_manager` - Mutable reference to the diagnostic manager where warnings are added.
///
/// # Returns
///
/// * `Ok(())` on success.
/// * `Err(ParserInternalError)` if the `RequireDef` node cannot be found or accessed.
///
/// # Example
///
/// ```rust
/// report_duplicate_requirements_warnings(
///     &ast.arena(),
///     require_def_id,
///     "domain.pddl",
///     &mut diagnostic_manager,
/// )?;
/// ```
pub fn report_duplicate_requirements_warnings(
    arena: &Arena<AstNode>,
    require_def_id: NodeId,
    source_name: &str,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), ParserInternalError> {
    // Try to get the RequireDef node by its ID. Return error if not found.
    let require_def_node = arena.try_node(require_def_id)?;

    // Create a HashSet to track seen requirements and a Vec to collect duplicates.
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();

    // Iterate over all children of the RequireDef node.
    for &child_id in require_def_node.children() {
        // Get the child node reference by its ID.
        let child = arena.get_node(child_id).unwrap();

        // Attempt to parse the child node as a requirement.
        if let Ok(req) = child.try_requirement() {
            // If this requirement was already seen, add it to duplicates.
            if !seen.insert(req) {
                duplicates.push(req);
            }
        }
    }

    // If duplicates were found, create a diagnostic warning and add it to the manager.
    if !duplicates.is_empty() {
        let warning = new_duplicate_requirement_warning(
            duplicates,
            require_def_node.span(),
            source_name,
        );
        diagnostic_manager.add_diagnostic(warning);
    }

    // Return Ok if everything went fine.
    Ok(())
}


/// Creates a diagnostic warning for one or more duplicate requirement declarations.
///
/// This function generates a diagnostic warning indicating that one or more
/// `:requirement` entries were declared multiple times in a PDDL domain definition. The warning
/// includes the names of the duplicated requirements, the source span (location) where the duplication
/// was detected, and the filename for context.
///
/// # Arguments
///
/// * `duplicate_requirements` - A vector of duplicated requirements detected in the domain.
/// * `span` - The source span (location) of the duplicated requirement(s).
/// * `source` - The name of the source file being analyzed.
///
/// # Returns
///
/// * A `Diagnostic` instance representing the duplicate requirement warning, ready to be
///   submitted to a diagnostic manager.
///
/// # Example
///
/// ```rust
/// let diagnostic = new_duplicate_requirement_warning(
///     vec![requirement1.clone(), requirement2.clone()],
///     &span,
///     "domain.pddl",
/// );
/// diagnostic_manager.add_diagnostic(diagnostic);
/// ```
pub fn new_duplicate_requirement_warning(
    duplicate_requirements: Vec<Requirement>,
    span: &Span,
    source: &str,
) -> Diagnostic {
    Diagnostic::new(
        DiagnosticKind::DuplicateRequirementWarning {
            duplicate_requirements,
        },
        Provider::Normalizer,
        source.to_string(),
        span.clone(),
    )
}

/// Removes duplicate requirement children from the `RequireDef` node in the arena.
///
/// Returns `Ok(true)` if any duplicates were removed, otherwise `Ok(false)`.
///
/// # Arguments
/// * `arena_mut` - Mutable reference to the arena containing the AST nodes.
/// * `require_def_id` - The `NodeId` of the `RequireDef` node.
///
/// # Errors
/// Returns an error if the node with `require_def_id` or its children cannot be accessed mutably.
pub fn remove_requirement_duplicates(
    arena_mut: &mut Arena<AstNode>,
    require_def_id: NodeId,
) -> Result<bool, ParserInternalError> {
    // Get mutable reference to RequireDef node
    let require_def_node_mut = arena_mut.try_node_mut(require_def_id)?;

    // Copy the children IDs to avoid mutable borrow conflicts
    let old_children = require_def_node_mut.children().to_vec();

    // Prepare new children vector and a set to track seen requirements
    let mut seen = HashSet::new();
    let mut new_children = Vec::with_capacity(old_children.len());

    // Track whether any duplicates were removed
    let mut modified = false;

    // Iterate over children and keep only unique requirements
    for &child_id in &old_children {
        let child = arena_mut.try_node_mut(child_id)?;
        match child.try_requirement() {
            Ok(req) if seen.insert(req) => new_children.push(child_id),
            Ok(_) => modified = true, // duplicate found and skipped
            Err(_) => new_children.push(child_id), // not a requirement, keep it
        }
    }

    // Re-borrow RequireDef node mutably to set filtered children
    let require_def_node_mut = arena_mut.try_node_mut(require_def_id)?;
    require_def_node_mut.set_children(new_children);

    Ok(modified)
}
