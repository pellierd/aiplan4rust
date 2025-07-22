//! Error types for operations on arena-based tree structures.
//!
//! This module defines the [`ArenaError`] enum, representing possible failure
//! modes when interacting with nodes in an arena-managed tree. These errors
//! include invalid accesses, missing nodes or parents, out-of-bounds indices,
//! and internal consistency violations.
//!
//! # Usage
//!
//! When performing operations on an arena tree (such as retrieving parents,
//! children, or nodes by ID), these errors signal problems like missing nodes,
//! incorrect assumptions about tree shape, or attempts to access invalid indices.
//!
//! Each variant provides context useful for debugging and error handling.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::core::arena::ArenaError;
//!
//! fn example(id: usize) -> Result<(), ArenaError> {
//!     if id == 0 {
//!         Err(ArenaError::node_not_found(id))
//!     } else {
//!         Ok(())
//!     }
//! }
//! ```
use thiserror::Error;

/// Represents errors that can occur when interacting with an [`Arena`]-based tree structure.
///
/// This enum covers a variety of error conditions including:
/// - Accessing non-existent nodes
/// - Invalid parent/child relationships
/// - Out-of-bounds indexing
/// - Violated internal invariants
#[derive(Error, Debug)]
pub enum ArenaError {
    /// A node with the specified ID was not found in the arena.
    ///
    /// This typically occurs when accessing a node by an invalid or previously deleted ID.
    ///
    /// # Parameters
    /// - `usize`: The missing node ID.
    #[error("Node with id {0} not found in the arena")]
    NodeNotFound(usize),

    /// A root node was expected but is missing.
    ///
    /// This can happen when attempting to access the root of a tree that was not initialized properly.
    #[error("Root ID is missing in the arena")]
    MissingRootId,

    /// A node ID was out of bounds for the current arena state.
    ///
    /// This indicates an attempt to access a node index beyond the maximum valid index (typically `arena.len() - 1`).
    ///
    /// # Fields
    /// - `id`: The requested node ID.
    /// - `max`: The highest valid ID in the arena.
    #[error("Node ID {id} is out of bounds (expected between 0 and {max})")]
    NodeIdOutOfBounds { id: usize, max: usize },

    /// A parent node was expected but not found.
    ///
    /// This error arises when an operation requires a parent (e.g., in `try_parent()`), but the node is actually a root.
    #[error("Expected parent node but found none")]
    MissingParent,

    /// A child was requested at an invalid index.
    ///
    /// This typically indicates a logic error where a child is assumed to exist at a certain index,
    /// but the actual number of children is lower.
    ///
    /// # Fields
    /// - `index`: The requested index.
    /// - `child_count`: The number of available children.
    #[error("Child index {index} is out of bounds: only {child_count} children exist")]
    ChildIndexOutOfBounds { index: usize, child_count: usize },

    /// A catch-all internal error used when no specific variant applies.
    ///
    /// This should only be used for unrecoverable or inconsistent internal state.
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl ArenaError {
    /// Creates a [`NodeNotFound`] error.
    pub fn node_not_found(id: usize) -> Self {
        Self::NodeNotFound(id)
    }

    /// Creates a [`MissingRootId`] error.
    pub fn missing_root_id() -> Self {
        Self::MissingRootId
    }

    /// Creates a [`NodeIdOutOfBounds`] error.
    pub fn node_id_out_of_bounds(id: usize, max: usize) -> Self {
        Self::NodeIdOutOfBounds { id, max }
    }

    /// Creates a [`MissingParent`] error.
    pub fn missing_parent() -> Self {
        Self::MissingParent
    }

    /// Creates a [`ChildIndexOutOfBounds`] error.
    pub fn child_index_out_of_bounds(index: usize, child_count: usize) -> Self {
        Self::ChildIndexOutOfBounds { index, child_count }
    }

    /// Creates an [`InternalError`] with a custom message.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError(message.into())
    }
}
