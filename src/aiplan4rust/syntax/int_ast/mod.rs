//! Syntax module for the `aiplan4rust` crate.
//!
//! This module provides the core Abstract Syntax Tree (AST) components and utilities
//! for representing and manipulating PDDL and HDDL syntax trees.
//!
//! # Overview
//!
//! - [`AstNode`]: Represents a node in the syntax tree.
//! - [`AstKind`]: Enumerates the kinds of syntax nodes.
//! - [`Ast`]: The full AST structure encapsulating the parsed tree.
//!
//! # Modules
//!
//! - `node`: Defines the AST node structure and its methods.
//! - `ast`: Provides the main AST type and related functionality.
//! - `kind`: Contains the enumeration of AST node kinds.
//! - `iterators`: Internal module for AST traversal iterators (not publicly exposed).
//!
//! # Re-exports
//!
//! For convenience, the following are re-exported publicly:
//! - `AstNode` (`node::Node`)
//! - `AstKind` (`kind::Kind`)
//! - `Ast` (`ast::Ast`)
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

pub mod content;
pub mod kind;
mod iterators;

pub use kind::Kind as IntAstKind;
pub use node::Node as IntAstNode;
pub use content::Content as AstContent;
pub use ast::Ast as IntAst;
