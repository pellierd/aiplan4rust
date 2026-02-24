//! Syntax support for the `aiplan4rust` crate.
//!
//! This module defines the tree components of the Abstract Syntax Tree (AST)
//! used to represent PDDL and HDDL domain and problem structures.
//!
//! # Structure
//!
//! This module is organized into several submodules:
//!
//! - [`node`]: Defines the internal structure of AST nodes.
//! - [`kind`]: Enumerates the kinds of AST nodes (e.g., keyword, identifier).
//! - [`content`]: Defines the data payload associated with AST nodes.
//!
//! # Traversal
//!
//! AST traversal uses the generic iterators provided by the underlying arena allocator,
//! which are publicly accessible through the [`syntax_tree`] field of the [`Ast`] struct.
//! This includes iterators such as:
//!
//! - Pre-order traversal (parent before children).
//! - Post-order traversal (children before parent).
//!
//! These iterators enable efficient walking of the [`AstNode`] tree.
//!
//! # Re-exports
//!
//! To simplification access to key types, several items are re-exported:
//!
//! - [`AstNode`] — alias of `node::AstNode`
//! - [`AstKind`] — alias of `kind::Kind`
//! - [`AstContent`] — alias of `content::Content`
//! - [`Ast`] — the full parsed AST structure defined in `ast`
//! - [`AstError`] — error types related to AST operations
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::syntax::{Ast, AstNode, AstKind};
//!
//! let ast: Ast = /* ... */;
//! let root = ast.root();
//!
//! // Traversal using iterators publicly exposed via syntax_tree
//! for (node, depth) in ast.syntax_tree().preorder() {
//!     println!("{}- {:?}", "  ".repeat(depth), node.kind());
//! }
//! ```
//!
//! # See Also
//!
//! - [`AstNode`] for individual AST nodes.
//! - [`AstKind`] for node kind classifications.
//! - [`AstContent`] for node payloads.
//! - [`AstError`] for error handling related to AST.
//! - [`syntax_tree`] field in [`Ast`] for access to arena and iterators.
//!

pub mod content;
pub mod kind;
pub mod ast;
pub mod node;
pub mod renderer;
pub mod error;

// Public re-exports
pub use kind::Kind as AstKind;
pub use content::Content as AstContent;
pub use ast::Ast;
pub use error::AstError;
pub use node::AstNode;
