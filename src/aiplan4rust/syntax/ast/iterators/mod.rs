//! Iterators for traversing tree-like structures using different depth-first strategies.
//!
//! This module provides two iterator implementations for walking over nodes in an
//! abstract syntax tree (AST) or similar hierarchical data structures:
//!
//! - [`PreorderIterator`]: Visits the current node before its children (pre-order traversal).
//! - [`PostorderIterator`]: Visits the children before the current node (post-order traversal).
//!
//! # Examples
//!
//! ```rust
//! use aiplan4rust::syntax::ast::PreorderIterator;
//!
//! // Assuming `root` is the root node of your tree:
//! let preorder_iter = PreorderIterator::new(&root);
//! for node in preorder_iter {
//!     println!("{:?}", node);
//! }
//! ```
//!
//! ```rust
//! use aiplan4rust::syntax::ast::PostorderIterator;
//!
//! // Assuming `root` is the root node of your tree:
//! let postorder_iter = PostorderIterator::new(&root);
//! for node in postorder_iter {
//!     println!("{:?}", node);
//! }
//! ```
//!
//! # Notes
//!
//! These iterators are designed to be used with tree structures where each node
//! can have zero or more child nodes. They abstract away the traversal logic,
//! providing a simple interface to visit nodes in a specific order.

mod preorder;
mod postorder;

pub use preorder::PreorderIterator;
pub use postorder::PostorderIterator;
