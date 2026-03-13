//! Module `arena_error`
//!
//! This module defines the [`ArenaError`] enum, representing all possible error
//! conditions that can arise when working with an [`Arena`]-based tree structure.
//!
//! The errors include issues such as missing nodes, invalid parent-child relationships,
//! out-of-bounds accesses, and internal invariant violations.
//!
//! # Error Variants
//!
//! - [`NodeNotFound`]: Node with a given ID does not exist in the arena.
//! - [`MissingRootId`]: Expected root node ID is missing.
//! - [`RootNodeNotFound`]: Root node ID is set but node not found.
//! - [`NodeIdOutOfBounds`]: Requested node ID is outside the valid range.
//! - [`MissingParent`]: Expected parent node is absent (node is root).
//! - [`ChildIndexOutOfBounds`]: Requested child index exceeds number of children.
//!
//! # Usage
//!
//! This error either_type is used throughout arena node operations to provide precise
//! feedback on failure modes, aiding in debugging and robust error handling.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::core::arena::error::ArenaError;
//!
//! fn example() -> Result<(), ArenaError> {
//!     Err(ArenaError::node_not_found(42))
//! }
//! ```
//!
//! [`Arena`]: crate::aiplan4rust::core::arena::Arena

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

    /// The root node was expected but not found.
    ///
    /// This can happen when the root ID is set but the corresponding node is absent from the arena.
    #[error("Root node with id {0} not found in the arena")]
    RootNodeNotFound(usize),

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
    /// This typically indicates a ops error where a child is assumed to exist at a certain index,
    /// but the actual number of children is lower.
    ///
    /// # Fields
    /// - `index`: The requested index.
    /// - `child_count`: The number of available children.
    #[error("Child index {index} is out of bounds: only {child_count} children exist")]
    ChildIndexOutOfBounds { index: usize, child_count: usize },
}

impl ArenaError {

    /// Captures the current call site and backtrace for debugging purposes.
    #[track_caller]
    fn capture(self) -> Self {
        if log::log_enabled!(log::Level::Debug) {
            let caller = std::panic::Location::caller();
            let bt = std::backtrace::Backtrace::force_capture();

            log::debug!(
                "Arena error captured at {file}:{line}:{col}\n\
                 [Error] {error:?}\n\
                 [Stack Trace]\n{trace}",
                file = caller.file(),
                line = caller.line(),
                col = caller.column(),
                error = self,
                trace = bt
            );
        }
        self
    }

    /// Creates a [`NodeNotFound`] error.
    #[track_caller]
    pub fn node_not_found(id: usize) -> Self {
        Self::NodeNotFound(id).capture()
    }

    /// Creates a [`MissingRootId`] error.
    #[track_caller]
    pub fn missing_root_id() -> Self {
        Self::MissingRootId.capture()
    }

    /// Creates a [`RootNodeNotFound`] error.
    #[track_caller]
    pub fn root_node_not_found(id: usize) -> Self {
        Self::RootNodeNotFound(id).capture()
    }

    /// Creates a [`NodeIdOutOfBounds`] error.
    #[track_caller]
    pub fn node_id_out_of_bounds(id: usize, max: usize) -> Self {
        Self::NodeIdOutOfBounds { id, max }.capture()
    }

    /// Creates a [`MissingParent`] error.
    #[track_caller]
    pub fn missing_parent() -> Self {
        Self::MissingParent.capture()
    }

    /// Creates a [`ChildIndexOutOfBounds`] error.
    #[track_caller]
    pub fn child_index_out_of_bounds(index: usize, child_count: usize) -> Self {
        Self::ChildIndexOutOfBounds { index, child_count }.capture()
    }
}
