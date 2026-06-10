//! Module `validation::error`
//!
//! This module defines the errors that can occur during structural validation of an AST (Abstract Syntax Tree).
//!
//! It mainly checks that the AST’s structure respects certain constraints,
//! such as the expected number of children, child node kinds, presence of nodes, etc.
//!
//! **Note:** This validation is **partial** and **not exhaustive**.
//! It does not cover all possible validity aspects of an AST,
//! including semantic consistency, logic, or other domain-specific rules.
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

use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::syntax::ast::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::ast::tree::NodeId;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind, AstNode};
use thiserror::Error;

/// Represents errors that can occur during AST validation.
///
/// This enum covers structural validation errors such as mismatched child counts,
/// unexpected child kinds, missing nodes, unexpected content, and invalid node kinds.
/// It also supports a generic `Custom` error for miscellaneous cases.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Wraps errors originating from arena allocation or manipulation.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// A parent-child relationship inconsistency detected within the arena structure.
    #[error(
        "Structural inconsistency: child node {child_id:?} claims to have parent {actual_parent:?} instead of expected parent {expected_parent:?}"
    )]
    StructuralInconsistency {
        child_id: NodeId,
        actual_parent: Option<NodeId>,
        expected_parent: NodeId,
    },

    /// The number of children of a node is not exactly the expected count.
    #[error(
        "Wrong number of children: expected {expected}, found {found} (parent: kind={}, content={}, span={})",
        parent.kind(), parent.content(), parent.span()
    )]
    ChildrenArityMismatch {
        expected: usize,
        found: usize,
        parent: AstNode,
    },

    /// The number of children of a node is outside a specified inclusive range.
    #[error(
        "Children count out of range: expected {}–{}, found {} (parent: kind={}, content={}, span={})",
        expected_min, expected_max, found, parent.kind(), parent.content(), parent.span()
    )]
    ChildrenArityOutOfRange {
        expected_min: usize,
        expected_max: usize,
        found: usize,
        parent: AstNode,
    },

    /// A child node at a specific index has an unexpected kind.
    #[error(
        "Unexpected child kind at index {index}: expected {:?}, found {} (parent: kind={}, content={}, span={})",
        expected, found, parent.kind(), parent.content(), parent.span()
    )]
    UnexpectedChildKind {
        index: usize,
        expected: Vec<AstKind>,
        found: AstKind,
        parent: AstNode,
    },

    /// A child node expected by its ID is missing from the AST.
    #[error(
        "Missing child node with ID {} (parent: kind={}, content={}, span={})",
        child_id, parent.kind(), parent.content(), parent.span()
    )]
    MissingChildNode { child_id: NodeId, parent: AstNode },

    /// A node contains unexpected content that does not match expectations.
    #[error(
        "Unexpected content in node: found {}, span={} (kind={}, content={})",
        found, node.span(), node.kind(), node.content()
    )]
    UnexpectedNodeContent { found: AstContent, node: AstNode },

    /// A node has an invalid kind that should not occur in a well-formed AST.
    #[error(
        "Invalid node kind encountered: kind={}, span={} (content={})",
        found.kind(), found.span(), found.content()
    )]
    InvalidNodeKind { found: AstNode },

    /// The AST contains a cycle, i.e., it is not a valid tree.
    #[error("Cycle detected in AST: the structure is not a tree")]
    CycleDetected,

    /// The AST is empty or has no root node.
    #[error("Missing root node: a PDDL AST must contain at least a 'domain' or a 'problem'")]
    MissingRoot,

    /// The root node of the AST has an invalid kind (expected Domain or Problem).
    #[error("Invalid root node kind: expected Domain or Problem, found {found}")]
    InvalidRoot { found: AstKind },

    /// A custom validation error with a free-form message.
    #[error("{0}")]
    Custom(String),
}

impl ValidationError {
    /// Creates a `StructuralInconsistency` error when an arena link is corrupted.
    ///
    /// # Parameters
    /// - `child_id`: The ID of the node that has an incorrect parent pointer.
    /// - `actual_parent`: The actual parent ID currently stored in the child node.
    /// - `expected_parent`: The ID of the parent that was iterating over this child.
    #[track_caller]
    pub fn structural_inconsistency(
        child_id: NodeId,
        actual_parent: Option<NodeId>,
        expected_parent: NodeId,
    ) -> Self {
        ValidationError::StructuralInconsistency {
            child_id,
            actual_parent,
            expected_parent,
        }
        .trace()
    }

    /// Creates a `ChildrenArityMismatch` error.
    ///
    /// # Parameters
    /// - `expected`: The exact number of children that was expected.
    /// - `found`: The actual number of children found in the AST node.
    /// - `parent`: The AST node whose children were being validated.
    #[track_caller]
    pub fn children_arity_mismatch(expected: usize, found: usize, parent: AstNode) -> Self {
        ValidationError::ChildrenArityMismatch {
            expected,
            found,
            parent,
        }
        .trace()
    }

    /// Creates a `ChildrenArityOutOfRange` error.
    ///
    /// # Parameters
    /// - `expected_min`: The minimum number of children allowed (inclusive).
    /// - `expected_max`: The maximum number of children allowed (inclusive).
    /// - `found`: The actual number of children found.
    /// - `parent`: The AST node whose children were being validated.
    #[track_caller]
    pub fn children_arity_out_of_range(
        expected_min: usize,
        expected_max: usize,
        found: usize,
        parent: AstNode,
    ) -> Self {
        ValidationError::ChildrenArityOutOfRange {
            expected_min,
            expected_max,
            found,
            parent,
        }
        .trace()
    }

    /// Creates an `UnexpectedChildKind` error.
    ///
    /// # Parameters
    /// - `index`: The index of the child node with the unexpected kind.
    /// - `expected`: A list of allowed `AstKind` values for that position.
    /// - `found`: The actual `AstKind` of the child node.
    /// - `parent`: The parent AST node containing the invalid child.
    #[track_caller]
    pub fn unexpected_child_kind(
        index: usize,
        expected: Vec<AstKind>,
        found: AstKind,
        parent: AstNode,
    ) -> Self {
        ValidationError::UnexpectedChildKind {
            index,
            expected,
            found,
            parent,
        }
        .trace()
    }

    /// Creates a `MissingChildNode` error.
    ///
    /// # Parameters
    /// - `child_id`: The node ID of the expected but missing child.
    /// - `parent`: The parent AST node that was expected to have this child.
    #[track_caller]
    pub fn missing_child_node(child_id: NodeId, parent: AstNode) -> Self {
        ValidationError::MissingChildNode { child_id, parent }.trace()
    }

    /// Creates an `UnexpectedNodeContent` error.
    ///
    /// # Parameters
    /// - `found`: The actual content of the node that did not match expectations.
    /// - `node`: The AST node containing the unexpected content.
    #[track_caller]
    pub fn unexpected_node_content(found: AstContent, node: AstNode) -> Self {
        ValidationError::UnexpectedNodeContent { found, node }.trace()
    }

    /// Creates an `InvalidNodeKind` error.
    ///
    /// # Parameters
    /// - `found`: The AST node with a kind that is considered invalid in the current context.
    #[track_caller]
    pub fn invalid_node_kind(found: AstNode) -> Self {
        ValidationError::InvalidNodeKind { found }
    }

    /// Creates a `CycleDetected` error.
    #[track_caller]
    pub fn cycle_detected() -> Self {
        ValidationError::CycleDetected.trace()
    }

    /// Creates a `MissingRoot` error.
    #[track_caller]
    pub fn missing_root() -> Self {
        ValidationError::MissingRoot.trace()
    }

    /// Creates an `InvalidRoot` error.
    ///
    /// # Parameters
    /// - `found`: The actual kind of the root node that was deemed invalid.
    #[track_caller]
    pub fn invalid_root(found: AstKind) -> Self {
        ValidationError::InvalidRoot { found }.trace()
    }

    /// Creates a generic `Custom` validation error.
    ///
    /// # Parameters
    /// - `msg`: A human-readable message describing the error.
    #[track_caller]
    pub fn custom<T: Into<String>>(msg: T) -> Self {
        ValidationError::Custom(msg.into()).trace()
    }
}

impl Traceable for ValidationError {}
