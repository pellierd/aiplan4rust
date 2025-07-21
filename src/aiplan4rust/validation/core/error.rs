//! Module `validation::error`
//!
//! This module defines the errors that can occur during structural validation of an AST (Abstract Syntax Tree).
//!
//! It mainly checks that the AST’s structure respects certain constraints,
//! such as the expected number of children, child node kinds, presence of nodes, etc.
//!
//! **Note:** This validation is **partial** and **not exhaustive**.
//! It does not cover all possible validity aspects of an AST,
//! including semantic consistency, normalization, or other domain-specific rules.
//!
//! For more advanced validation, additional mechanisms should be employed.
//!
//! # Examples of detected errors:
//! - Incorrect number of children for a given node.
//! - Unexpected kind of child node at a given position.
//! - Missing child node.
//! - Unexpected content in a node.
//!
//! # Usage
//!
//! Validation functions typically return a [`ValidationError`] on failure,
//! providing detailed information about the cause and context of the error.

use std::fmt;
use crate::aiplan4rust::syntax::tree::NodeId;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind, AstNode};

/// Represents errors that can occur during AST validation.
///
/// This enum covers structural validation errors such as mismatched child counts,
/// unexpected child kinds, missing nodes, unexpected content, and invalid node kinds.
/// It also supports a generic `Custom` error for miscellaneous cases.
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// The number of children of a node is not exactly the expected count.
    ///
    /// Contains the expected count, the actual count found,
    /// and the parent node where the mismatch occurred.
    ChildrenArityMismatch {
        expected: usize,
        found: usize,
        parent: AstNode,
    },

    /// The number of children of a node is outside a specified inclusive range.
    ///
    /// Contains the minimum and maximum expected counts,
    /// the actual count found, and the parent node.
    ChildrenArityOutOfRange {
        expected_min: usize,
        expected_max: usize,
        found: usize,
        parent: AstNode,
    },

    /// A child node at a specific index has an unexpected kind.
    ///
    /// Contains the child index, the list of expected kinds,
    /// the kind found, and the parent node.
    UnexpectedChildKind {
        index: usize,
        expected: Vec<AstKind>,
        found: AstKind,
        parent: AstNode,
    },

    /// A child node expected by its ID is missing from the AST.
    ///
    /// Contains the missing child node's ID and its parent node.
    MissingChildNode {
        child_id: NodeId,
        parent: AstNode,
    },

    /// A node contains unexpected content that does not match expectations.
    ///
    /// Contains the content found and the node where it was found.
    UnexpectedNodeContent {
        found: AstContent,
        node: AstNode,
    },

    /// A node has an invalid kind that should not occur in a well-formed AST.
    ///
    /// Contains the offending node.
    InvalidNodeKind {
        found: AstNode,
    },

    /// A custom validation error with a free-form message.
    Custom(String),
}

impl fmt::Display for ValidationError {
    /// Formats the validation error as a human-readable string.
    ///
    /// This implementation shows detailed information including the node kind,
    /// content, span, and the position of the error relative to the AST structure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Helper function to show basic info about a node’s parent.
        fn parent_info(node: &AstNode) -> String {
            format!(
                "parent: kind={}, content={}, span={}",
                node.kind(),
                node.content(),
                node.span()
            )
        }

        match self {
            ValidationError::ChildrenArityMismatch { expected, found, parent } => {
                write!(
                    f,
                    "Wrong number of children: expected {}, found {} ({})",
                    expected,
                    found,
                    parent_info(parent)
                )
            }

            ValidationError::ChildrenArityOutOfRange { expected_min, expected_max, found, parent } => {
                write!(
                    f,
                    "Children count out of range: {}–{} expected, found {} ({})",
                    expected_min,
                    expected_max,
                    found,
                    parent_info(parent)
                )
            }

            ValidationError::UnexpectedChildKind { index, expected, found, parent } => {
                write!(
                    f,
                    "Unexpected child kind at index {}: expected {:?}, found {:?} ({})",
                    index,
                    expected,
                    found,
                    parent_info(parent)
                )
            }

            ValidationError::MissingChildNode { child_id, parent } => {
                write!(
                    f,
                    "Missing child node with ID {} ({})",
                    child_id,
                    parent_info(parent)
                )
            }

            ValidationError::UnexpectedNodeContent { found, node } => {
                write!(
                    f,
                    "Unexpected content in node: found {}, span={} ({})",
                    found,
                    node.span(),
                    parent_info(node)
                )
            }

            ValidationError::InvalidNodeKind { found } => {
                write!(
                    f,
                    "Invalid node kind encountered: {}, span={} ({})",
                    found.kind(),
                    found.span(),
                    parent_info(found)
                )
            }

            ValidationError::Custom(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}
