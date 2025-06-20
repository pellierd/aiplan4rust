use std::collections::HashSet;

use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::{Ast, AstContent};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::elements::Requirement;
use crate::aiplan4rust::syntax::Span;

/// Normalizes the requirement declarations by removing duplicates from the `RequireDef` node.
///
/// This function assumes that the input AST is already **valid** and structurally correct.
/// Specifically, the `RequireDef` node—if present—must only contain well-formed `Requirement` nodes
/// as children.
///
/// The normalization does not traverse the full AST. Instead, it directly locates the `RequireDef`
/// node (if any), which is expected to contain all `:requirement` declarations. It removes duplicate
/// requirements (i.e., those with the same key) and emits a diagnostic warning for each duplicate
/// found and removed.
///
/// This pass is **self-contained** and **stateless**—it does not depend on any other normalization
/// pass and does not affect or rely on their outcomes. It can safely be run independently at any
/// point, provided the AST is valid.
///
/// # Arguments
///
/// * `ast_old` - A mutable reference to the AST that may contain a `RequireDef`.
/// * `diagnostic_manager` - A manager used to report warnings when duplicate requirements are found.
///
/// # Returns
///
/// * `Ok(true)` if at least one duplicate requirement was removed.
/// * `Ok(false)` if no duplicates were found or if no `RequireDef` node exists.
/// * `Err(ParserInternalError)` if the AST structure is not as expected (e.g., invalid node kinds).
///
/// # Assumptions
///
/// * The AST must be valid (i.e., conforming to the grammar and invariants expected by the parser).
/// * The `RequireDef` node—if present—must contain only `Requirement` children.
/// * No other normalization passes are required before or after this one.
/// * This function is deterministic and has no side effects outside its scope.
///
/// # Panics
///
/// This function may panic if the `RequireDef` node contains unexpected children (e.g., non-`Requirement` nodes),
/// which is considered a violation of AST validity and a programming error.
///
/// # Example
///
/// ```rust
/// let mut ast_old = parse_source_code(source)?;
/// let mut diagnostics = DiagnosticManager::new();
/// let changed = normalize_require_def(&mut ast_old, &mut diagnostics)?;
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

    // Set to track seen requirements to detect duplicates
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();

    // Retain only unique Requirement nodes, remove duplicates and report warnings
    require_def.children_mut().retain(|child| {
        match child.try_requirement() {
            Ok(r) if seen.insert(r) => true,
            Ok(r) => {
                duplicates.push(r);
                modified = true;
                false
            }
            Err(_) => true,
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
/// * `ast_old` - A mutable reference to the AST to search.
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
    if let Some(require_def_node) = ast.find_node_of_kind_mut(AstKind::RequireDef) {
        for child in require_def_node.children() {
            if !matches!(child.kind(), AstKind::Requirement) {
                return Err(ParserInternalError::new(format!(
                    "Invalid child node in RequireDef: expected Requirement, found {:?}",
                    child.kind()
                )));
            }
        }
        Ok(Some(require_def_node))
    } else {
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
