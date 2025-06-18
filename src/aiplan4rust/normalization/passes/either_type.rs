use std::collections::HashSet;

use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast_old::AstNodeOld;
use crate::aiplan4rust::syntax::ast_old::AstKindOld;
use crate::aiplan4rust::syntax::ast_old::AstOld;
use crate::aiplan4rust::syntax::Span;

/// Normalizes all `Type` nodes in the given AST by removing duplicate `PrimitiveType` children.
///
/// This function starts from the root node and recursively normalizes the entire syntax tree.
/// It delegates the actual work to `normalize_typed_list_node`.
///
/// # Parameters
///
/// - `ast_old`: A mutable reference to the `Ast` to normalize.
///
/// # Returns
///
/// - `Ok(true)` if any duplicates were removed and the AST was modified.
/// - `Ok(false)` if no modifications were necessary.
/// - `Err(_)` if a validation error occurred during normalization.
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::syntax::ast_old::Ast;
/// use crate::aiplan4rust::parser::ParserInternalError;
///
/// let mut ast_old = Ast::new(...);
/// match normalize_typed_list(&mut ast_old) {
///     Ok(modified) => {
///         if modified {
///             println!("AST was normalized and modified.");
///         } else {
///             println!("No changes were necessary.");
///         }
///     }
///     Err(e) => eprintln!("Normalization error: {}", e),
/// }
/// ```
pub fn normalize_either_type(
    ast: &mut AstOld,
    diagnostic_manager: &mut  DiagnosticManager
) -> Result<bool, ParserInternalError> {
    let source = ast.source_name().to_string();
    // Access the mutable root node of the AST and delegate normalization
    normalize_either_type_node(ast.root_mut(), &source, diagnostic_manager)
}

/// Normalizes `Type` nodes by removing duplicate `PrimitiveType` children.
///
/// This function performs two main tasks:
///
/// 1. It validates that all children of a `Type` node are `PrimitiveType` nodes by calling
///    `assert_type_validity`. If validation fails, it returns an error.
/// 2. It removes duplicate `PrimitiveType` children within each `Type` node.
///
/// The function applies recursively to all descendants of the given node.
///
/// # Parameters
///
/// - `node`: A mutable reference to the AST node to normalize.
///
/// # Returns
///
/// - `Ok(true)` if the tree was modified (duplicates were removed).
/// - `Ok(false)` if no changes were made.
/// - `Err(_)` if a validation error occurred (invalid children in a `Type` node).
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::syntax::ast::AstNode;
/// use crate::aiplan4rust::parser::ParserInternalError;
///
/// let mut ast_node: AstNode = /* construct your AST node */;
///
/// match normalize_either_type_node(&mut ast_node) {
///     Ok(modified) => {
///         if modified {
///             println!("The AST was modified.");
///         } else {
///             println!("No changes were necessary.");
///         }
///     }
///     Err(e) => eprintln!("Validation error: {}", e),
/// }
/// ```
fn normalize_either_type_node(
        node: &mut AstNodeOld,
        source: &String,
        diagnostic_manager: &mut DiagnosticManager
) -> Result<bool, ParserInternalError> {
    // First, validate the current node to ensure all children are PrimitiveType
    assert_either_type_validity(node)?;

    // Track whether any modifications have been made to this subtree
    let mut modified = false;

    // If the current node is of kind Type, proceed to remove duplicates
    if let AstKindOld::Type = node.kind() {
        // Create a set to track seen PrimitiveType names
        let mut seen = HashSet::new();
        let mut duplicates = Vec::new();

        // Retain only unique PrimitiveType children, removing duplicates
        node.children_mut().retain(|child| {
            if let AstKindOld::PrimitiveType(name) = child.kind() {
                if seen.insert(name.clone()) {
                    true // Keep this unique PrimitiveType child
                } else {
                    duplicates.push(name.clone());
                    modified = true; // Duplicate found and removed
                    false // Remove this duplicate child
                }
            } else {
                true // Keep non-PrimitiveType children (should be none after validation)
            }
        });
        if !duplicates.is_empty() {
            // Report a diagnostic warning for the removed duplicates
            report_duplicate_either_type_warning(
                duplicates,
                source,
                node.span(),
                diagnostic_manager,
            );
        }
    }

    // Recursively normalize all child nodes
    for child in node.children_mut() {
        if normalize_either_type_node(child, source, diagnostic_manager)? {
            modified = true; // If any child was modified, mark this node as modified
        }
    }

    // Return whether the AST subtree was modified
    Ok(modified)
}

/// Reports a diagnostic warning when duplicate types are found in a typed list declaration.
///
/// This function is called during normalization to notify the user that a declaration contained
/// repeated types (e.g., `x y - (either t1 t1)`), which were automatically removed.
///
/// # Parameters
///
/// - `declaration`: The `Declaration` node where duplicate types were found and removed.
/// - `duplicates`: A vector of duplicate type names that were removed.
/// - `source`: The filename or source identifier where the declaration originated.
/// - `diagnostic_manager`: A mutable reference to the diagnostic manager responsible for tracking
///   diagnostics.
fn report_duplicate_either_type_warning(
    duplicates: Vec<String>,
    source: &str,
    span: &Span,
    diagnostic_manager: &mut DiagnosticManager,
) {
    // Construct the diagnostic warning with detailed metadata
    let diagnostic = Diagnostic::new(
        DiagnosticKind::DuplicateEitherTypeWarning {
            duplicate_types: duplicates,
        },
        Provider::Normalizer,
        source.to_string(),
        span.clone(),
    );

    // Submit the warning to the diagnostic manager
    diagnostic_manager.add_diagnostic(diagnostic);
}

/// Asserts that all children of a `Type` node are `PrimitiveType` nodes.
///
/// This function checks that every child node of the given AST node, if it is of kind `Type`,
/// is a `PrimitiveType`. If any child is not a `PrimitiveType`, the function returns a
/// `ParserInternalError`.
///
/// # Parameters
///
/// - `node`: A reference to the AST node to validate.
///
/// # Errors
///
/// Returns a `ParserInternalError` if any child of the `Type` node is not a `PrimitiveType`.
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::syntax::ast::{AstNode, AstKind};
/// use crate::aiplan4rust::syntax::parser::ParserInternalError;
///
/// // Assume `node` is an AstNode
/// if let Err(e) = assert_either_type_validity(&node) {
///     eprintln!("Validation error: {}", e);
/// }
/// ```
pub fn assert_either_type_validity(node: &AstNodeOld) -> Result<(), ParserInternalError> {
    if let AstKindOld::Type = node.kind() {
        for child in node.children() {
            if !matches!(child.kind(), AstKindOld::PrimitiveType(_)) {
                return Err(ParserInternalError::new(format!(
                    "Expected only PrimitiveType in Type node, found: {:?}",
                    child.kind()
                )));
            }
        }
    }
    Ok(())
}
