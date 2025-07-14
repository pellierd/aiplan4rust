use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::validation::common;
use crate::aiplan4rust::validation::common::checks::ContentKind;
use crate::WellFormedError;

/// Checks that a symbol node has identifier content and no children.
///
/// # Arguments
/// * `node` - The node to check.
///
pub fn check_symbol(node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_content(node, ContentKind::Ident)?;
    common::checks::check_children_count(node.children().len(), 0, node)?;
    Ok(())
}

/// Checks that the node represents a well-formed number (float)
/// with no children.
///
/// # Arguments
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the content is not a float or if the node has any children.
pub fn check_number(node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_content(node, ContentKind::Float)?;
    common::checks::check_children_count(node.children().len(), 0, node)?;
    Ok(())
}

/// Checks that the node represents a well-formed requirement
/// with no children.
///
/// # Arguments
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the content is not a requirement or if the node has any children.
pub fn check_requirement(node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_content(node, ContentKind::Requirement)?;
    common::checks::check_children_count(node.children().len(), 0, node)?;
    Ok(())
}
