//! Syntax module for the `aiplan4rust` crate.
//!
//! This module provides the core Abstract Syntax Tree (AST) components and utilities
//! for representing and manipulating PDDL and HDDL syntax trees.
//!
//! # Overview
//!
//! - [`AstNode`]: Represents a node in the syntax tree.
//! - [`AstKind`]: Enumerates the kinds of syntax nodes.
//! - [`AstOld`]: The full AST structure encapsulating the parsed tree.
//!
//! # Modules
//!
//! - `node`: Defines the AST node structure and its methods.
//! - `ast_old`: Provides the main AST type and related functionality.
//! - `kind`: Contains the enumeration of AST node kinds.
//! - `iterators`: Internal module for AST traversal iterators (not publicly exposed).
//!
//! # Re-exports
//!
//! For convenience, the following are re-exported publicly:
//! - `AstNode` (`node::Node`)
//! - `AstKind` (`kind::Kind`)
//! - `Ast` (`ast_old::Ast`)
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::syntax::{AstNode, AstKind, Ast};
//!
//! // Example usage with AST nodes and kinds here...
//! ```

pub mod node;
pub mod ast;

pub mod kind;
mod iterators;
pub mod serialize;

pub use kind::AstKindOld;
pub use node::AstNodeOld;
pub use ast::AstOld;
