use std::collections::HashSet;

use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::{AstContent, AstNode};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::Span;

/// Normalizes all `Type` nodes in the given AST by removing duplicate `PrimitiveType` children.
///
/// This function starts from the root node and recursively normalizes the entire syntax tree.
/// It delegates the actual work to `normalize_typed_list_node`.
///
/// # Parameters
///
/// - `ast`: A mutable reference to the `Ast` to normalize.
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
/// use crate::aiplan4rust::syntax::ast::Ast;
/// use crate::aiplan4rust::parser::ParserInternalError;
///
/// let mut ast = Ast::new(...);
/// match normalize_typed_list(&mut ast) {
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
    ast: &mut Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let root = ast.root_mut();        // emprunt mutable ici
     // emprunt immuable sur context seulement

    let (modified, warnings) = collect_either_type_info(root)?;

    report_either_type_duplicates(warnings, ast, diagnostic_manager);

    Ok(modified)
}

fn collect_either_type_info(
    root: &mut AstNode,
) -> Result<(bool, Vec<(Vec<usize>, Span)>), ParserInternalError> {
    let mut modified = false;
    let mut warnings = Vec::new();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        assert_either_type_validity(node)?;

        if let AstKind::Type = node.kind() {
            let mut seen = HashSet::new();
            let mut dups = Vec::new();

            node.children_mut().retain(|child| {
                if let AstKind::PrimitiveType = child.kind() {
                    if let AstContent::Ident(id) = child.content() {
                        if seen.insert(*id) {
                            true
                        } else {
                            dups.push(*id);
                            modified = true;
                            false
                        }
                    } else {
                        true
                    }
                } else {
                    true
                }
            });

            if !dups.is_empty() {
                warnings.push((dups, node.span().clone()));
            }
        }

        for child in node.children_mut() {
            stack.push(child);
        }
    }

    Ok((modified, warnings))
}

/// 2ᵉ passe : pour chaque entrée `(dups, span)` on appelle ton reporter.
fn report_either_type_duplicates(
    warnings: Vec<(Vec<usize>, Span)>,
    ast: &Ast,
    diagnostic_manager: &mut DiagnosticManager,
) {
    for (duplicates_ids, span) in warnings {
        // Conversion des ids en String, on filtre les ids qui n'ont pas de correspondance
        let duplicates: Vec<String> = duplicates_ids
            .into_iter()
            .filter_map(|id| ast.context().get_str(id).map(|s| s.to_string()))
            .collect();

        report_duplicate_either_type_warning(
            duplicates,
            ast.source_name(),
            &span,
            diagnostic_manager,
        );
    }
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
pub fn assert_either_type_validity(node: &AstNode) -> Result<(), ParserInternalError> {
    if let AstKind::Type = node.kind() {
        for child in node.children() {
            if !matches!(child.kind(), AstKind::PrimitiveType) {
                return Err(ParserInternalError::new(format!(
                    "Expected only PrimitiveType in Type node, found: {:?}",
                    child.kind()
                )));
            }
        }
    }
    Ok(())
}
