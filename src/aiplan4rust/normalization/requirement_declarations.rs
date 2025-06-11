use std::collections::HashSet;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::{Ast, AstKind, AstNode};
use crate::aiplan4rust::syntax::elements::Requirement;
use crate::aiplan4rust::syntax::Span;

/// Normalizes the requirement declarations by removing duplicates from the `RequireDef` node.
///
/// This function assumes the AST structure contains a specific node, typically called `RequireDef`,
/// which directly holds all `:requirement` declarations as its children. Instead of traversing the
/// entire AST, this function locates that node and removes duplicate requirements within it.
///
/// # Arguments
///
/// * `ast` - A mutable reference to the AST to be normalized.
/// * `diagnostic_manager` - The diagnostic manager to report warnings for duplicate requirements.
///
/// # Returns
///
/// Returns `Ok(true)` if at least one duplicate requirement was removed, `Ok(false)` otherwise.
/// Returns an error if the AST structure does not match expectations or other internal errors occur.
///
/// # Example
///
/// ```rust
/// let mut ast = ...; // your parsed AST
/// let mut diagnostics = DiagnosticManager::new();
/// let changed = normalize_requirement_declarations(ast, &mut diagnostics)?;
/// if changed {
///     println!("Duplicate requirements were removed.");
/// }
/// ```
/// Normalizes requirement declarations in the AST by removing duplicate requirements.
///
/// # Arguments
/// * `ast` - The mutable abstract syntax tree to process.
/// * `diagnostic_manager` - Manager to report warnings and diagnostics.
///
/// # Returns
/// * `Ok(true)` if any duplicate requirements were removed.
/// * `Ok(false)` if no duplicates were found or no `RequireDef` node exists.
/// * `Err(ParserInternalError)` if structural validation fails.
///
/// # Panics
/// This function will panic if the `RequireDef` node contains non-`Requirement` children,
/// which should never happen if the AST is valid.
pub fn normalize_requirement_declarations(
    ast: &mut Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    // Get the filename or default to "unknown"
    let filename = ast
        .filename()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    // Track whether any duplicates were removed
    let mut removed_any = false;

    // Find and validate the RequireDef node in the AST
    let require_def_node_mut = match find_valid_require_def_node_mut(ast)? {
        Some(node) => node,
        None => return Ok(false), // No RequireDef node found, nothing to normalize
    };

    // Set to track seen requirements to detect duplicates
    let mut seen = HashSet::new();

    // Retain only unique Requirement nodes, remove duplicates and report warnings
    require_def_node_mut.children_mut().retain(|child| {
        if let AstKind::Requirement(requirement) = child.kind() {
            // Check if this requirement is a duplicate
            let is_duplicate = !seen.insert(requirement.clone());

            if is_duplicate {
                // Report a warning about the duplicate requirement
                report_duplicate_requirement_declaration_warning(
                    requirement,
                    child.span(),
                    &filename,
                    diagnostic_manager,
                );
                removed_any = true;
                false // Remove duplicate from children
            } else {
                true // Keep unique requirement
            }
        } else {
            // This should never happen if the AST is valid and produced by the parser
            unreachable!("Non-Requirement node found in RequireDef children during duplicate removal");
        }
    });

    // Return whether any duplicates were removed
    Ok(removed_any)
}

/// Searches the AST for a `RequireDef` node and validates that all its children are `Requirement`
/// nodes.
///
/// # Arguments
/// * `ast` - The mutable AST to search.
///
/// # Returns
/// * `Ok(Some(&mut AstNode))` if a valid `RequireDef` node is found.
/// * `Ok(None)` if no `RequireDef` node is present.
/// * `Err(ParserInternalError)` if any child of the `RequireDef` is not a `Requirement`.
///
/// # Errors
/// Returns a `ParserInternalError` if an invalid node is found within the `RequireDef`.
fn find_valid_require_def_node_mut(
    ast: &mut Ast,
) -> Result<Option<&mut AstNode>, ParserInternalError> {
    // Search for the first RequireDef node among the root's children
    let require_def_node = ast
        .root_mut()
        .children_mut()
        .iter_mut()
        .find(|node| matches!(node.kind(), AstKind::RequireDef));

    // If a RequireDef node is found, validate its structure
    if let Some(require_def_node) = require_def_node {
        for child in require_def_node.children() {
            // All children must be Requirement nodes
            if !matches!(child.kind(), AstKind::Requirement(_)) {
                return Err(ParserInternalError::new(format!(
                    "Expected Requirement node in RequireDef, but found {:?} at span {:?}",
                    child.kind(),
                    child.span()
                )));
            }
        }
        // Return the dereferenced reference from Box<Node>
        Ok(Some(require_def_node.as_mut()))
    } else {
        // No RequireDef node found
        Ok(None)
    }
}

/// Reports a warning diagnostic when a duplicate requirement declaration is encountered.
///
/// This function creates and submits a diagnostic warning indicating that a specific
/// `:requirement` was declared more than once. The warning includes the requirement name,
/// its location in the source, and the filename for context.
///
/// # Arguments
///
/// * `requirement` - The requirement that is duplicated.
/// * `span` - The source span (location) of the duplicated requirement.
/// * `filename` - The name of the source file being analyzed.
/// * `diagnostic_manager` - The diagnostic manager used to record the warning.
///
/// # Example
///
/// ```rust
/// report_duplicate_requirement_declaration_warning(
///     &requirement,
///     &span,
///     "domain.pddl",
///     &mut diagnostic_manager,
/// );
/// ```
pub fn report_duplicate_requirement_declaration_warning(
    requirement: &Requirement,
    span: &Span,
    filename: &str,
    diagnostic_manager: &mut DiagnosticManager,
) {
    // Construct a diagnostic warning with details about the duplicate requirement
    let diagnostic = Diagnostic::new(
        DiagnosticKind::DuplicateRequirementDeclarationWarning {
            requirement: requirement.clone()
        },
        Provider::Normalizer,                // The source of this diagnostic
        filename.to_string(),              // Filename for context in the message
        span.clone(),                     // Location in source code of duplicate
    );

    // Add the diagnostic warning to the manager to report it later
    diagnostic_manager.add_diagnostic(diagnostic);
}
