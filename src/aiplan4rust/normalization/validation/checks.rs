//! Provides validation checks for specific AST node kinds related to typing and parameters.
//!
//! This module implements structural validation rules for nodes like `TypedItem`, `TypesDef`,
//! `ParametersDef`, and quantified logic (`Forall`, `Exists`). It also provides recursive
//! checks for typed lists, ensuring proper child kinds and arities according to language rules.
//!
//! # Overview
//!
//! - `check_typed_item`: Validates that a `TypedItem` node has either one or two children with correct kinds.
//! - `check_types_def`: Ensures `TypesDef` nodes have at least one child and that the first child is a `TypedList`.
//! - `check_parameters_def`: Verifies `ParametersDef` nodes have exactly one `TypedList` child.
//! - `check_quantified_expression`: Checks quantifier nodes have a `TypedList` and an expression as children.
//! - `check_typed_list_of`: Recursively validates a typed list subtree for proper node kinds and structure.
//!
//! # Usage
//!
//! These functions typically operate on references to the AST and specific nodes,
//! returning detailed errors if validation fails. They rely on common and logic
//! helper functions for checking children count, kinds, and retrieving nodes.
//!
//! # Errors
//!
//! All validation functions return `Result<(), WellFormedError>` or `Result<(), WellNormalizedError>`,
//! detailing the cause of any structural inconsistency encountered during validation.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::validation::logic::{check_typed_item, check_types_def};
//! # let ast = ...; // Your AST instance
//! # let node = ...; // Your AST node
//! if let Err(e) = check_typed_item(&ast, &node) {
//!     eprintln!("Invalid TypedItem node: {}", e);
//! }
//! ```

use crate::aiplan4rust::normalization::validation::WellNormalizedError;
use crate::aiplan4rust::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::syntax::ast::{Ast, AstKind, AstNode};
use crate::aiplan4rust::syntax::validation;
use crate::aiplan4rust::syntax::validation::checks::EXPRESSION;
use crate::aiplan4rust::syntax::validation::WellFormedError;

/// Checks that the given node of kind `TypedItem` has either one or two children:
///
/// - If it has **one child**, it must be one of:
///   `PrimitiveType`, `Constant`, `Variable`, or `AtomicFunctionSkeleton`.
///
/// - If it has **two children**:
///   - The first must be one of:
///     `PrimitiveType`, `Constant`, `Variable`, or `AtomicFunctionSkeleton`.
///   - The second must be of kind `Type`.
///
/// # Arguments
///
/// * `ast` - Reference to the AST containing the node.
/// * `node` - The AST node to validate.
///
/// # Errors
///
/// Returns an error if:
/// - The number of children is not 1 or 2.
/// - The children are not of the expected kinds.
///
/// # Examples
///
/// ```rust
/// let result = check_typed_item(&ast, &node);
/// if let Err(e) = result {
///     eprintln!("TypedItem node is invalid: {:?}", e);
/// }
/// ```
pub fn check_typed_item(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    validation::checks::check_children_count_range(children_len, 1, 2, node)?;

    match children_len {
        1 => {
            validation::checks::check_child_kind(
                ast,
                node,
                0,
                &[
                    AstKind::PrimitiveType,
                    AstKind::Object,
                    AstKind::Variable,
                    AstKind::AtomicFunctionSkeleton,
                ],
            )?;
            Ok(())
        }
        2 => {
            validation::checks::check_child_kind(
                ast,
                node,
                0,
                &[
                    AstKind::PrimitiveType,
                    AstKind::Object,
                    AstKind::Variable,
                    AstKind::AtomicFunctionSkeleton,
                ],
            )?;
            validation::checks::check_child_kind(ast, node, 1, &[AstKind::Type])
        }
        _ => unreachable!(),
    }
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
    validation::checks::check_min_children_count(node.arity(), 1, node)?;
    let typed_list = validation::checks::get_child_node(ast, node, 0)?;
    check_typed_list_of(ast, typed_list, &[AstKind::PrimitiveType])
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
    let children_len = node.arity();
    validation::checks::check_children_count(children_len, 1, node)?;
    let typed_list = validation::checks::get_child_node(ast, node, 0)?;
    check_typed_list_of(ast, typed_list, &[AstKind::Variable])
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
pub fn check_quantified_expression(ast: &Ast, node: &AstNode) -> Result<(), WellFormedError> {
    let children_len = node.arity();
    validation::checks::check_min_children_count(children_len, 2, node)?;
    let typed_list = validation::checks::get_child_node(ast, node, 0)?;
    check_typed_list_of(ast, typed_list, &[AstKind::Variable])?;
    validation::checks::check_child_kind(ast, node, 1, EXPRESSION)
}

/// Recursively checks that the given AST node and its descendants conform to the expected
/// structure for typed lists.
///
/// Specifically:
/// - If the node is of kind `TypedList`, verifies that all its children are `TypedItem` nodes.
/// - If the node is of kind `TypedItem`, validates that it has exactly two children:
///   the first being one of the expected kinds (such as a typed element like variable, constant,
///   primitive type_checker, or atomic formula skeleton), and the second a `Type` node.
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
            validation::checks::check_typed_list(ast, node)?;
        }
        AstKind::TypedItem => {
            // TypedItem should have exactly two children: a typed element (variable, constant, primitive type_checker,
            // or atomic formula skeleton) and a Type node
            check_typed_item(ast, node)?;
            return Ok(());
        }
        _ => {
            // If the node kind is unexpected here, return an error indicating invalid node kind
            return Err(WellNormalizedError::InvalidNodeKind {
                found: node.clone(),
            }
            .into());
        }
    }

    // For TypedList nodes, recursively check all children
    for child_id in children_ids {
        let child_node = validation::checks::get_node(ast, node, child_id.as_usize())?;
        // Recursive call with the same expected kinds
        check_typed_list_of(ast, child_node, expected)?;
    }

    Ok(())
}
