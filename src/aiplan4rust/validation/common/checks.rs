//! This module provides helper functions and constants to perform
//! structural validation of an Abstract Syntax Tree (AST) used in
//! the AIPlan4Rust project.
//!
//! The validation focuses on checking node kinds, children counts,
//! expected content kinds, and other well-formedness properties.
//!
//! Note: This validation is not exhaustive and does not guarantee
//! semantic correctness. It only ensures that the AST structure
//! meets basic well-formedness criteria.
//!
//! The constant `EXPRESSION` defines a set of `AstKind` variants
//! considered valid logic within the AST.
//!
//! # Usage
//! Use the provided functions to check specific properties of AST nodes,
//! including children kinds, counts, and content, returning detailed
//! `WellFormedError`s in case of validation failures.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::validation::common::WellFormedError;
use crate::aiplan4rust::syntax::ast::{Ast, AstContent, AstKind, AstNode};

/// Set of AST node kinds considered as valid logic.
pub const EXPRESSION: &[AstKind] = &[
    AstKind::Or,
    AstKind::And,
    AstKind::Not,
    AstKind::Imply,
    AstKind::Exists,
    AstKind::Forall,
    AstKind::AtomicFormula,
    AstKind::Preference,
    AstKind::When,
    AstKind::Comparison,
    AstKind::Assignment,
    AstKind::Arithmetic,
    AstKind::AtStart,
    AstKind::AtEnd,
    AstKind::Overall,
    AstKind::Always,
    AstKind::Sometime,
    AstKind::Within,
    AstKind::AtMostOnce,
    AstKind::SometimeAfter,
    AstKind::SometimeBefore,
    AstKind::AlwaysWithin,
    AstKind::HoldDuring,
    AstKind::HoldAfter,
    AstKind::TimedInitialLiteral,
    AstKind::LabeledTask,
    AstKind::Task,
    AstKind::TaskOrderingConstraint,
];

/// Enumeration representing the expected kind of content inside an AST node.
#[derive(Debug)]
pub enum ContentKind {
    /// Node with no content.
    None,
    /// Identifier content.
    Ident,
    /// Floating point number content.
    Float,
    /// Requirement content.
    Requirement,
    /// Binary comparison operator content.
    BinaryComp,
    /// Assignment operator content.
    AssignOp,
    /// Arithmetic operator content.
    ArithmeticOp,
    /// Optimization specification content.
    Optimization,
}

/// Checks that the child at a given index of a parent AST node has one of the expected kinds.
///
/// This validation ensures that the child node at the specified `index` within the `parent` node
/// matches one of the `expected_kinds`, which is important for enforcing structural constraints
/// on the AST.
///
/// # Arguments
/// * `ast` - The full AST, used to resolve child node references from the parent.
/// * `parent` - The parent node whose child is being validated.
/// * `index` - The index of the child in the parent's children list to check.
/// * `expected_kinds` - A slice of acceptable [`AstKind`]s that the child is expected to match.
///
/// # Errors
/// Returns a [`WellFormedError::UnexpectedChildKind`] if the actual kind of the child node
/// does not match any of the kinds listed in `expected_kinds`.
///
/// Also propagates any error returned by [`get_node`] if the child cannot be resolved from the AST.
pub fn check_child_kind(
    ast: &Ast,
    parent: &AstNode,
    index: usize,
    expected_kinds: &[AstKind],
) -> Result<(), WellFormedError> {
    let children = parent.children();

    let child = get_node(ast, parent, children[index].as_usize())?;
    let found = child.kind();

    if expected_kinds.contains(&found) {
        Ok(())
    } else {
        Err(WellFormedError::unexpected_child_kind(
            index,
            expected_kinds.to_vec(),
            found,
            parent.clone(),
        )
            .into())
    }
}

/// Checks that the number of children of a node is exactly equal to `expected`.
///
/// This validation is useful to enforce the arity of AST nodes, ensuring that
/// nodes have the exact number of children expected for their kind.
///
/// # Arguments
/// * `children_len` - The actual number of children the node has.
/// * `expected` - The required number of children for this node.
/// * `parent` - The AST node whose children are being validated.
///
/// # Errors
/// Returns a [`WellFormedError::ChildrenArityMismatch`] if `children_len` does not equal `expected`.
pub fn check_children_count(
    children_len: usize,
    expected: usize,
    parent: &AstNode,
) -> Result<(), WellFormedError> {
    if children_len != expected {
        Err(WellFormedError::children_arity_mismatch(
            expected,
            children_len,
            parent.clone(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that the number of children of a node is at least `min`.
///
/// This ensures that a node meets the minimum required arity for correct structure.
///
/// # Arguments
/// * `children_len` - The actual number of children the node has.
/// * `min` - The minimum required number of children for this node.
/// * `parent` - The AST node whose children are being validated.
///
/// # Errors
/// Returns a [`WellFormedError::ChildrenArityOutOfRange`] if `children_len` is less than `min`.
pub fn check_min_children_count(
    children_len: usize,
    min: usize,
    parent: &AstNode,
) -> Result<(), WellFormedError> {
    if children_len < min {
        Err(WellFormedError::children_arity_out_of_range(
            min,
            usize::MAX,
            children_len,
            parent.clone(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that the number of children lies within the inclusive range [`min`, `max`].
///
/// Ensures the node's arity falls within the accepted bounds for structural validation.
///
/// # Arguments
/// * `children_count` - The actual number of children the node has.
/// * `min` - The minimum allowed number of children (inclusive).
/// * `max` - The maximum allowed number of children (inclusive).
/// * `parent` - The AST node being validated.
///
/// # Errors
/// Returns a [`WellFormedError::ChildrenArityOutOfRange`] if `children_count` is less than `min` or greater than `max`.
pub fn check_children_count_range(
    children_count: usize,
    min: usize,
    max: usize,
    parent: &AstNode,
) -> Result<(), WellFormedError> {
    if children_count < min || children_count > max {
        Err(WellFormedError::children_arity_out_of_range(
            min,
            max,
            children_count,
            parent.clone(),
        ))
    } else {
        Ok(())
    }
}

/// Checks that the content of the `node` matches the expected `ContentKind`.
///
/// Validates that the node's content variant matches the expected kind,
/// ensuring semantic correctness of the AST node.
///
/// # Arguments
/// * `node` - The AST node to validate.
/// * `expected` - The expected content kind for this node.
///
/// # Errors
/// Returns a `WellFormedError::UnexpectedNodeContent` if the node's content does not match the expected kind.
pub fn check_content(node: &AstNode, expected: ContentKind) -> Result<(), WellFormedError> {
    match (node.content(), expected) {
        (AstContent::None, ContentKind::None) => Ok(()),
        (AstContent::Ident(_), ContentKind::Ident) => Ok(()),
        (AstContent::Number(_), ContentKind::Float) => Ok(()),
        (AstContent::Requirement(_), ContentKind::Requirement) => Ok(()),
        (AstContent::CompareOp(_), ContentKind::BinaryComp) => Ok(()),
        (AstContent::AssignOp(_), ContentKind::AssignOp) => Ok(()),
        (AstContent::ArithmeticOp(_), ContentKind::ArithmeticOp) => Ok(()),
        (AstContent::OptimizationOp(_), ContentKind::Optimization) => Ok(()),

        (found, _) => Err(WellFormedError::unexpected_node_content(
            found.clone(),
            node.clone(),
        )),
    }
}

/// Immediately returns an `InvalidNodeKind` error for the given `found` node.
///
/// # Arguments
/// * `found` - The AST node that has an invalid kind.
///
/// # Errors
/// Always returns a `WellFormedError::InvalidNodeKind` containing the given node.
pub fn throw_invalid(found: &AstNode) -> Result<(), WellFormedError> {
    Err(WellFormedError::invalid_node_kind(found.clone()))
}

/// Attempts to retrieve a child node by `node_id` from the `ast` arena.
///
/// # Arguments
/// * `ast` - The AST containing the nodes.
/// * `parent` - The parent node from which we expect the child.
/// * `node_id` - The ID of the child node to retrieve.
///
/// # Errors
/// Returns `WellFormedError::MissingChildNode` if the node with `node_id` is not found in the AST.
pub fn get_node<'a>(
    ast: &'a Ast,
    parent: &AstNode,
    node_id: usize,
) -> Result<&'a AstNode, WellFormedError> {
    match ast.syntax_tree().get_node(NodeId::new(node_id)) {
        Some(node) => Ok(node),
        None => Err(WellFormedError::missing_child_node(
            NodeId::new(node_id),
            parent.clone(),
        )),
    }
}
/// Attempts to retrieve the child node at `child_index` from `parent` node in the `ast` arena.
///
/// # Arguments
/// * `ast` - The AST containing the nodes.
/// * `parent` - The parent node whose child is to be retrieved.
/// * `child_index` - The index of the child node to retrieve.
///
/// # Errors
/// Returns `WellFormedError::MissingChildNode` if the child node at `child_index` does not exist.
pub fn get_child_node<'a>(
    ast: &'a Ast,
    parent: &AstNode,
    child_index: usize,
) -> Result<&'a AstNode, WellFormedError> {
    // Retrieve the child node ID at the given index
    let child_id = parent.children()[child_index].as_usize();
    get_node(ast, parent, child_id)
}


/// Checks that *all* children of `parent` have kinds included in `expected_kinds`.
///
/// This is equivalent to calling [`check_children_kind_from`] with `start_index` set to 0.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the nodes.
/// * `parent` - The parent node whose children will be checked.
/// * `expected_kinds` - A slice of allowed kinds for the children.
///
/// # Errors
/// Returns the first encountered `WellFormedError`, typically
/// a `WellFormedError::UnexpectedChildKind` if any child’s kind
/// is not among `expected_kinds`.
pub fn check_all_children_kind(
    ast: &Ast,
    parent: &AstNode,
    expected_kinds: &[AstKind],
) -> Result<(), WellFormedError> {
    check_children_kind_from(ast, parent, 0, expected_kinds)
}

/// Checks that all children of `parent` starting from `start_index` have kinds included in `expected_kinds`.
///
/// # Arguments
/// * `ast` - Reference to the AST containing the nodes.
/// * `parent` - The parent node whose children will be checked.
/// * `start_index` - The index from which to start checking children (inclusive).
/// * `expected_kinds` - A slice of allowed kinds for the children.
///
/// # Errors
/// Returns the first encountered `WellFormedError`, typically
/// a `WellFormedError::UnexpectedChildKind` if any child’s kind
/// is not among `expected_kinds`.
pub fn check_children_kind_from(
    ast: &Ast,
    parent: &AstNode,
    start_index: usize,
    expected_kinds: &[AstKind],
) -> Result<(), WellFormedError> {
    let children = parent.children();

    for i in start_index..children.len() {
        check_child_kind(ast, parent, i, expected_kinds)?;
    }

    Ok(())
}
