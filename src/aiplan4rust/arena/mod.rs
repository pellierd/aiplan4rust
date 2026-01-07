//! Arena module for managing syntax nodes in tree structures.
//!
//! This module provides common abstractions and utilities for working with
//! arena-allocated syntax trees, where nodes are identified by `NodeId`
//! and stored efficiently in a contiguous arena structure.
//!
//! # Key components:
//!
//! - [`ArenaTree`]: The arena data structure managing nodes and their hierarchy.
//! - [`NodeId`]: Unique identifier for nodes within the arena.
//! - [`ArenaNode`]: Trait that syntax node types implement to interact with the arena.
//! - [`NodeRef`]: Lightweight, non-owning reference to a node and its ID.
//! - [`BaseNode`]: A basic implementation of `ArenaNode` with common functionality.
//! - [`ArenaError`]: Error types related to arena operations.
//!
//! # Submodules:
//!
//! - `node_id`: Definition and implementation of `NodeId`.
//! - `iter`: Iterators over nodes and children within the arena.
//! - `node`: Core trait `ArenaNode` and related abstractions.
//! - `node_ref`: Types for immutable and mutable references to nodes.
//! - `base_node`: A concrete node implementation with standard behavior.
//! - `error`: Definitions of error types for arena operations.
//!
//! # Usage
//!
//! Use the re-exported types from this module to interact with arena nodes:
//!
//! ```rust
//! use crate::arena::{ArenaTree, NodeId, ArenaNode, NodeRef, BaseNode, ArenaError};
//! ```
pub mod arena;
pub mod node_id;
pub mod iter;
pub mod node;
pub mod node_ref;
pub mod base_node;
pub mod error;

pub use node_id::NodeId;
pub use arena::ArenaTree;
pub use node::ArenaNode;
pub use node_ref::NodeRef;
pub use base_node::BaseNode;
pub use error::ArenaError;
