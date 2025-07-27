//! Expression module for the AI Plan Rust project.
//!
//! This module provides the core types and functionality to represent,
//! manipulate, and analyze logical and planning expressions within
//! the system. It includes representations of expression nodes,
//! kinds, contents, transformations, errors, and higher-level expression
//! arenas.
//!
//! # Overview
//!
//! - [`content`]: Defines the `Content` enum, representing the payload
//!   or data attached to expression nodes.
//! - [`node`]: Defines `ExprNode`, the fundamental building block
//!   representing a single expression node in a syntax tree.
//! - [`kind`]: Defines the `Kind` enum representing the different
//!   kinds of expression nodes (logical operators, symbols, predicates, etc.).
//! - [`transform`]: Contains utilities and functions for transforming
//!   or rewriting expressions.
//! - [`expr`]: Defines the `Expr` type_checker, a wrapper around an expression
//!   arena (syntax tree) that holds `ExprNode` instances and provides
//!   expression-level operations.
//! - [`error`]: Defines error types related to expression parsing,
//!   conversion, and processing.
//!
//! # Type aliases
//!
//! - `ExprId`: Alias for `NodeId`, representing node identifiers
//!   within expression arenas.
//!
//! # Usage example
//!
//! ```rust
//! use crate::aiplan4rust::lir::expr::{Expr, ExprNode, ExprKind, ExprContent};
//!
//! // Create a new expression node with kind and content
//! let node = ExprNode::new(ExprKind::And, ExprContent::None, None);
//! // Build or manipulate expressions...
//! ```
//!
//! This modular design promotes clear separation of concerns,
//! making the expression representation extensible and easier to maintain.

use crate::aiplan4rust::syntax::tree::NodeId;

pub mod content;
pub mod node;
pub mod kind;
pub mod transform;
pub mod expr;
pub mod error;

pub use content::Content as ExprContent;
pub use node::ExprNode;
pub use kind::Kind as ExprKind;
pub use expr::Expr;
pub use error::ExprError;

/// Alias for expression node identifiers within the syntax tree.
pub type ExprId = NodeId;
