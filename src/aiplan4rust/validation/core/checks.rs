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
//! considered valid expressions within the AST.
//!
//! # Usage
//! Use the provided functions to check specific properties of AST nodes,
//! including children kinds, counts, and content, returning detailed
//! `WellFormedError`s in case of validation failures.

use crate::aiplan4rust::syntax::tree::NodeId;
use crate::aiplan4rust::validation::core::WellFormedError;
use crate::aiplan4rust::syntax::ast::{Ast, AstContent, AstKind, AstNode};

/// Set of AST node kinds considered as valid expressions.
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
    AstKind::FComp,
    AstKind::Assign,
    AstKind::Operation,
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
    AstKind::TaggedTask,
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

/// Checks that the child at `index` of `parent` node has one of the `expected_kinds`.
///
/// # Errors
/// Returns `WellFormedError::UnexpectedChildKind` if the child's kind is not in `expected_kinds`.
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
        Err(WellFormedError::UnexpectedChildKind {
            index,
            expected: expected_kinds.to_vec(),
            found,
            parent: parent.clone(),
        })
    }
}

/// Checks that the number of children equals `expected`.
///
/// # Errors
/// Returns `WellFormedError::ChildrenArityMismatch` if `children_len` does not equal `expected`.
pub fn check_children_count(
    children_len: usize,
    expected: usize,
    parent: &AstNode,
) -> Result<(), WellFormedError> {
    if children_len != expected {
        Err(WellFormedError::ChildrenArityMismatch {
            expected,
            found: children_len,
            parent: parent.clone(),
        })
    } else {
        Ok(())
    }
}

/// Checks that there are at least `min` children.
///
/// # Errors
/// Returns `WellFormedError::ChildrenArityMismatch` if `children_len` is less than `min`.
pub fn check_min_children_count(
    children_len: usize,
    min: usize,
    parent: &AstNode,
) -> Result<(), WellFormedError> {
    if children_len < min {
        Err(WellFormedError::ChildrenArityMismatch {
            expected: min,
            found: children_len,
            parent: parent.clone(),
        })
    } else {
        Ok(())
    }
}

/// Checks that the number of children lies within the inclusive range [`min`, `max`].
///
/// # Errors
/// Returns `WellFormedError::ChildrenArityOutOfRange` if `children_count` is outside the range.
pub fn check_children_count_range(
    children_count: usize,
    min: usize,
    max: usize,
    parent: &AstNode,
) -> Result<(), WellFormedError> {
    if children_count < min || children_count > max {
        Err(WellFormedError::ChildrenArityOutOfRange {
            expected_min: min,
            expected_max: max,
            found: children_count,
            parent: parent.clone(),
        })
    } else {
        Ok(())
    }
}

/// Checks that the content of the `node` matches the expected `ContentKind`.
///
/// # Errors
/// Returns `WellFormedError::UnexpectedNodeContent` if the content kind does not match.
pub fn check_content(node: &AstNode, expected: ContentKind) -> Result<(), WellFormedError> {
    match (node.content(), expected) {
        (AstContent::None, ContentKind::None) => Ok(()),
        (AstContent::Ident(_), ContentKind::Ident) => Ok(()),
        (AstContent::Float(_), ContentKind::Float) => Ok(()),
        (AstContent::Requirement(_), ContentKind::Requirement) => Ok(()),
        (AstContent::BinaryComp(_), ContentKind::BinaryComp) => Ok(()),
        (AstContent::AssignOp(_), ContentKind::AssignOp) => Ok(()),
        (AstContent::ArithmeticOp(_), ContentKind::ArithmeticOp) => Ok(()),
        (AstContent::Optimization(_), ContentKind::Optimization) => Ok(()),

        (found, _) => Err(WellFormedError::UnexpectedNodeContent {
            found: found.clone(),
            node: node.clone(),
        }),
    }
}

/// Immediately returns an `InvalidNodeKind` error for the given `found` node.
pub fn throw_invalid(found: &AstNode) -> Result<(), WellFormedError> {
    Err(WellFormedError::InvalidNodeKind {
        found: found.clone(),
    })
}

/// Attempts to retrieve a child node by `node_id` from the `ast` arena.
///
/// # Errors
/// Returns `WellFormedError::MissingChildNode` if the node is not found.
pub fn get_node<'a>(
    ast: &'a Ast,
    parent: &AstNode,
    node_id: usize,
) -> Result<&'a AstNode, WellFormedError> {
    match ast.syntax_tree().get_node(NodeId::new(node_id)) {
        Some(node) => Ok(node),
        None => Err(WellFormedError::MissingChildNode {
            child_id: NodeId::new(node_id),
            parent: parent.clone(),
        }),
    }
}

/// Attempts to retrieve a child node by `node_id` from the `ast` arena.
///
/// # Errors
/// Returns `WellFormedError::MissingChildNode` if the node is not found.
pub fn get_child_node<'a>(
    ast: &'a Ast,
    parent: &AstNode,
    child_index: usize,
) -> Result<&'a AstNode, WellFormedError> {
    get_node(ast, parent, parent.children()[child_index].as_usize())
}


/// Checks that *all* children of `parent` have kinds included in `expected_kinds`.
///
/// Equivalent to calling `check_children_kind_from` with `start_index` 0.
///
/// # Errors
/// Returns the first encountered `WellFormedError`.
pub fn check_all_children_kind(
    ast: &Ast,
    parent: &AstNode,
    expected_kinds: &[AstKind],
) -> Result<(), WellFormedError> {
    check_children_kind_from(ast, parent, 0, expected_kinds)
}

/// Checks that all children of `parent` starting from `start_index` have kinds included in `expected_kinds`.
///
/// # Errors
/// Returns the first encountered `WellFormedError`.
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
