use std::collections::HashSet;

use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::elements::Requirement;
use crate::aiplan4rust::syntax::Span;

/// Normalizes the requirement declarations by removing duplicates from the `RequireDef` node.
///
/// This function assumes the AST contains a specific node, typically named `RequireDef`,
/// which holds all `:requirement` declarations as its children. Instead of traversing the
/// entire AST, it locates that node and removes duplicate requirements within it.
///
/// Duplicate requirements are removed, and a warning diagnostic is reported for each duplicate.
///
/// # Arguments
///
/// * `ast` - A mutable reference to the AST to be normalized.
/// * `diagnostic_manager` - The diagnostic manager used to report warnings for duplicates.
///
/// # Returns
///
/// Returns:
/// * `Ok(true)` if at least one duplicate requirement was removed.
/// * `Ok(false)` if no duplicates were found or if no `RequireDef` node exists.
/// * `Err(ParserInternalError)` if the AST structure does not match expectations or other internal
///   errors occur.
///
/// # Panics
///
/// This function will panic if the `RequireDef` node contains children that are not `Requirement`
/// nodes,
/// which should never happen if the AST is valid.
///
/// # Example
///
/// ```rust
/// let mut ast = ...; // your parsed AST
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
    // Get the filename or default to "unknown"
    let source_name = ast.source_name().clone();

    // Track whether any duplicates were removed
    let mut modified = false;

    // Find and validate the RequireDef node in the AST
    let require_def = match find_require_def(ast)? {
        Some(node) => node,
        None => return Ok(false), // No RequireDef node found, nothing to normalize
    };

    assert_require_def_validity(require_def)?;

    // Set to track seen requirements to detect duplicates
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();


    // Retain only unique Requirement nodes, remove duplicates and report warnings
    require_def.children_mut().retain(|child| {
        if let AstKind::Requirement(requirement) = child.kind() {
            // Check if this requirement is a duplicate
            if seen.insert(requirement.clone()) {
                true // Keep this unique PrimitiveType child
            } else {
                duplicates.push(requirement.clone());
                modified = true; // Duplicate found and removed
                false // Remove this duplicate child
            }
        } else {
            true // Keep non-PrimitiveType children (should be none after validation)
        }
    });

    if !duplicates.is_empty() {
        report_duplicate_requirement_warning(
            duplicates,
            require_def.span(),
            &source_name,
            diagnostic_manager);
    }
    // Return whether any duplicates were removed
    Ok(modified)
}

/// Searches the AST for a `RequireDef` node and validates that all its children are `Requirement`
/// nodes.
///
/// This function traverses the root children of the AST looking for the first node of kind
/// `RequireDef`. If found, it asserts that every child of this node is a `Requirement`.
///
/// # Arguments
///
/// * `ast` - A mutable reference to the AST to search.
///
/// # Returns
///
/// * `Ok(Some(&mut AstNode))` if a valid `RequireDef` node is found and all its children are valid.
/// * `Ok(None)` if no `RequireDef` node is present in the AST.
/// * `Err(ParserInternalError)` if any child of the `RequireDef` node is not a `Requirement`.
///
/// # Errors
///
/// Returns a `ParserInternalError` if an invalid child node is found within the `RequireDef`.
fn find_require_def(
    ast: &mut Ast,
) -> Result<Option<&mut AstNode>, ParserInternalError> {
    // Search for the first RequireDef node among the root's children
    let require_def_node = ast
        .root_mut()
        .children_mut()
        .iter_mut()
        .find(|node| matches!(node.kind(), AstKind::RequireDef));

    // If a RequireDef node is found, validate its children
    if let Some(require_def_node) = require_def_node {
        // Validate that all children are Requirement nodes
        for child in require_def_node.children() {
            if !matches!(child.kind(), AstKind::Requirement(_)) {
                return Err(ParserInternalError::new(format!(
                    "Invalid child node in RequireDef: expected Requirement, found {:?}",
                    child.kind()
                )));
            }
        }
        // Return the mutable reference to the valid RequireDef node
        Ok(Some(require_def_node.as_mut()))
    } else {
        // No RequireDef node found
        Ok(None)
    }
}

/// Reports a warning diagnostic when one or more duplicate requirement declarations are encountered.
///
/// This function creates and submits a diagnostic warning indicating that one or more
/// `:requirement` entries were declared multiple times in a PDDL domain definition. The warning
/// includes the names of the duplicated requirements, the source span (location) where the duplication
/// was detected, and the filename for context.
///
/// # Arguments
///
/// * `duplicate_requirements` - A vector of duplicated requirements detected in the domain.
/// * `span` - The source span (location) of the duplicated requirement(s).
/// * `source` - The name of the source file being analyzed.
/// * `diagnostic_manager` - The diagnostic manager used to record and report the warning.
///
/// # Example
///
/// ```rust
/// report_duplicate_requirement_declaration_warning(
///     vec![requirement1.clone(), requirement2.clone()],
///     &span,
///     "domain.pddl",
///     &mut diagnostic_manager,
/// );
/// ```
pub fn report_duplicate_requirement_warning(
    duplicate_requirements: Vec<Requirement>,
    span: &Span,
    source: &str,
    diagnostic_manager: &mut DiagnosticManager,
) {
    // Construct a diagnostic warning with details about the duplicate requirements
    let diagnostic = Diagnostic::new(
        DiagnosticKind::DuplicateRequirementWarning {
            duplicate_requirements
        },
        Provider::Normalizer,      // The source of this diagnostic
        source.to_string(),        // Filename for context in the message
        span.clone(),             // Location in source code of duplicate(s)
    );

    // Add the diagnostic warning to the manager to report it later
    diagnostic_manager.add_diagnostic(diagnostic);
}

/// Asserts that all children of a `RequireDef` node are `Requirement` nodes.
///
/// This function checks that every child node of the given AST node, if it is of kind `RequireDef`,
/// is a `Requirement`. If any child is not a `Requirement`, the function returns a
/// `ParserInternalError`.
///
/// # Parameters
///
/// - `node`: A reference to the AST node to validate.
///
/// # Errors
///
/// Returns a `ParserInternalError` if any child of the `RequireDef` node is not a `Requirement`.
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::syntax::ast::{AstNode, AstKind};
/// use crate::aiplan4rust::syntax::parser::ParserInternalError;
///
/// // Assume `node` is an AstNode
/// if let Err(e) = assert_require_def_validity(&node) {
///     eprintln!("Validation error: {}", e);
/// }
/// ```
pub fn assert_require_def_validity(node: &AstNode) -> Result<(), ParserInternalError> {
    if let AstKind::RequireDef = node.kind() {
        for child in node.children() {
            if !matches!(child.kind(), AstKind::Requirement(_)) {
                return Err(ParserInternalError::new(format!(
                    "Expected only Requirement nodes in RequireDef node, found: {:?}",
                    child.kind()
                )));
            }
        }
    }
    Ok(())
}
