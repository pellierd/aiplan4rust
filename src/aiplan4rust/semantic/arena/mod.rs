//! The `arena` module provides a memory-efficient arena allocator for abstract syntax trees (ASTs).
//!
//! # Overview
//!
//! This crate contains:
//! - [`arena`]: The main data structure managing a contiguous storage of AST nodes.
//! - [`node`]: Definition of the `Node` struct representing individual AST nodes.
//! - [`iterators`]: Various tree traversal iterators (preorder, postorder), some yielding node
//!   indices.
//!
//! # Key Types
//!
//! - [`ArenaAst`]: Alias for [`Arena`] from the `arena` module. Manages nodes and their
//!   relationships.
//! - [`ArenaAstNode`]: Alias for [`Node`] from the `node` module. Represents a single AST node.
//!
//! # Features
//!
//! - Efficient storage of AST nodes in a flat vector with parent and child indices.
//! - Iterators to traverse the AST in preorder or postorder, optionally returning node indices.
//! - Convenience methods for adding nodes and constructing arenas from existing ASTs.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode};
//!
//! let mut arena = ArenaAst::new();
//! let root_id = arena.add(/* kind */, /* span */, None);
//! let child_id = arena.add(/* kind */, /* span */, Some(root_id));
//!
//! for (idx, node) in arena.preorder_with_index() {
//!     println!("Node #{}: {:?}", idx, node.kind());
//! }
//! ```
//!
//! [`arena`]: crate::aiplan4rust::semantic::arena::arena
//! [`node`]: crate::aiplan4rust::semantic::arena::node
//! [`iterators`]: crate::aiplan4rust::semantic::arena::iterators
//! [`Arena`]: crate::aiplan4rust::semantic::arena::arena::Arena
//! [`Node`]: crate::aiplan4rust::semantic::arena::node::Node

pub mod arena;
pub mod node;
pub mod iterators;

pub mod id;

pub use node::Node as ArenaAstNode;
pub use arena::Arena as ArenaAst;
pub use id::Id as NodeId;
