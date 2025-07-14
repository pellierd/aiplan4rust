use crate::aiplan4rust::syntax::ast::{Ast, AstKind, AstNode};
use crate::aiplan4rust::validation::common::WellNormalizedError;
use crate::aiplan4rust::validation::{common, normalization, syntax};
use crate::aiplan4rust::validation::common::checks::EXPRESSION;
use crate::WellFormedError;

/// Checks that the given node of kind `TypedItem` has either one or two children:
/// - If one child, it must be of kind `TypedItemElements`.
/// - If two children, the first must be one of `PrimitiveType`, `Constant`, `Variable`, or
///  `AtomicFunctionSkeleton`, and the second must be of kind `Type`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if:
/// - The number of children is not 1 or 2.
/// - The children are not of the expected kinds.
///
/// # Examples
/// ```
/// let result = check_typed_item(&ast, &node);
/// if let Err(e) = result {
///     eprintln!("TypedItem node is invalid: {:?}", e);
/// }
/// ```
pub fn check_typed_item(ast: &Ast, node: &AstNode) -> Result<(), WellNormalizedError> {
    let children_len = node.children().len();
    common::checks::check_children_count(children_len, 2, node)?;
    common::checks::check_child_kind(ast, node, 0, &[
        AstKind::PrimitiveType,
        AstKind::Constant,
        AstKind::Variable,
        AstKind::AtomicFunctionSkeleton
    ])?;
    common::checks::check_child_kind(ast, node, 1, &[AstKind::Type])
}

/// Checks that the given node of kind `TypesDef` has at least one child,
/// and that the first child is of kind `TypedList`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node has fewer than one child,
/// or if the first child is not of kind `TypedList`.
pub fn check_types_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    common::checks::check_min_children_count(node.children().len(), 1, node)?;
    let typed_list = common::checks::get_child_node(ast, node, 0)?;
    normalization::checks::check_typed_list_of(ast, typed_list, &[AstKind::PrimitiveType])
}

/// Checks that the given node of kind `ParametersDef` has exactly one child,
/// and that this child is of kind `TypedList`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have exactly one child,
/// or if the child is not of kind `TypedList`.
pub fn check_parameters_def(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.children().len();
    common::checks::check_children_count(children_len, 1, node)?;
    let typed_list = common::checks::get_child_node(ast, node, 0)?;
    normalization::checks::check_typed_list_of(ast, typed_list, &[AstKind::Variable])
}

/// Checks that the given node is a quantifier (`Forall` or `Exists`) with:
/// - at least two children,
/// - the first child being a `TypedList`,
/// - the second child being an expression.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
/// Returns an error if the node does not have at least two children,
/// or if the children do not have the expected kinds.
pub fn check_quantifier_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.children().len();
    common::checks::check_min_children_count(children_len, 2, node)?;
    let typed_list = common::checks::get_child_node(ast, node, 0)?;
    normalization::checks::check_typed_list_of(ast, typed_list, &[AstKind::Variable])?;
    common::checks::check_child_kind(ast, node, 1, EXPRESSION)
}

/// Recursively checks that the given AST node and its descendants conform to the expected
/// structure for typed lists.
///
/// Specifically:
/// - If the node is of kind `TypedList`, verifies that all its children are `TypedItem` nodes.
/// - If the node is of kind `TypedItem`, validates that it has exactly two children:
///   the first being one of the expected kinds (such as a typed element like variable, constant,
///   primitive type, or atomic formula skeleton), and the second a `Type` node.
/// - Returns an error if the node kind is neither `TypedList` nor `TypedItem`.
///
/// # Arguments
/// * `ast` - Reference to the entire AST.
/// * `node` - The current AST node to check.
/// * `expected` - Slice of `AstKind` specifying the expected kinds for typed elements.
///
/// # Errors
/// Returns an error if the node is of an unexpected kind, or if the structural checks fail.
///
/// # Notes
/// This function recursively descends into children nodes to validate the entire subtree.
pub fn check_typed_list_of(
    ast: &Ast,
    node: &AstNode,
    expected: &[AstKind],
) -> Result<(), WellNormalizedError> {
    // Get the list of child node IDs of the current node
    let children_ids = node.children();

    // Match on the kind of the current node to apply the appropriate checks
    match node.kind() {
        AstKind::TypedList => {
            // If it's a TypedList, check that all its children are TypedItem nodes
            syntax::checks::check_typed_list(ast, node)?;
        }
        AstKind::TypedItem => {
            // TypedItem should have exactly two children: a typed element (variable, constant, primitive type,
            // or atomic formula skeleton) and a Type node
            normalization::checks::check_typed_item(ast, node)?;
            return Ok(())
        }
        _ => {
            // If the node kind is unexpected here, return an error indicating invalid node kind
            return Err(WellNormalizedError::InvalidNodeKind { found: node.clone() }.into());
        }
    }

    // For TypedList nodes, recursively check all children
    for child_id in children_ids {
        let child_node = common::checks::get_node(ast, node, child_id.as_usize())?;
        // Recursive call with the same expected kinds
        syntax::checks::check_typed_list_of(ast, child_node, expected)?;
    }

    Ok(())
}
