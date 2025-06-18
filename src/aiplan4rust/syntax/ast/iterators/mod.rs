//! Tree traversal iterators for abstract syntax trees (ASTs) and similar structures.
//!
//! This module provides two depth-first traversal iterators for hierarchical data:
//!
//! - [`PreorderIter`]: Visits each node **before** its children (pre-order).
//! - [`PostorderIter`]: Visits each node **after** its children (post-order).
//!
//! # Examples
//!
//! ```rust
//! use aiplan4rust::syntax::iterators::PreorderIter;
//!
//! // Assuming `root` is the root node of your tree:
//! let preorder_iter = PreorderIter::new(&root);
//! for node in preorder_iter {
//!     println!("{:?}", node);
//! }
//! ```
//!
//! ```rust
//! use aiplan4rust::syntax::iterators::PostorderIter;
//!
//! let postorder_iter = PostorderIter::new(&root);
//! for node in postorder_iter {
//!     println!("{:?}", node);
//! }
//! ```
//!
//! # Notes
//!
//! These iterators are generic over tree-like structures where each node has zero
//! or more children. They encapsulate traversal logic, making it easy to explore
//! nodes in a consistent and controlled manner.

pub mod preorder;
pub mod postorder;

pub use preorder::PreorderIter;
pub use postorder::PostorderIter;
