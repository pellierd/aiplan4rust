//! Iterators for traversing nodes in an `ArenaTree` using preorder and postorder strategies.
//!
//! This module provides the main entry point to preorder and postorder iterators.
//! It re-exports `PreorderIter` and `PostorderIter` from their respective submodules,
//! allowing users to easily import and use these iterators for tree traversal.
//!
//! # Modules
//!
//! - [`preorder`]: Contains the `PreorderIter` struct and related functionality.
//! - [`postorder`]: Contains the `PostorderIter` struct and related functionality.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::core::arena::iterators::{PreorderIter, PostorderIter};
//!
//! let arena = ...; // Your ArenaTree instance
//! let root = arena.root_id().unwrap();
//!
//! for (id, depth, node) in PreorderIter::new(&arena, root) {
//!     println!("Preorder visit node {:?} at depth {}", id, depth);
//! }
//!
//! for (id, depth, node) in PostorderIter::new(&arena, root) {
//!     println!("Postorder visit node {:?} at depth {}", id, depth);
//! }
//! ```

pub mod preorder;
pub mod postorder;

pub use preorder::PreorderIter;
pub use postorder::PostorderIter;
