//! Syntax support for the `aiplan4rust` crate.
//!
//! This module defines the core components of the Abstract Syntax Tree (AST)
//! used to represent PDDL and HDDL domain and problem structures.
//!
//! # Structure
//!
//! This module is organized into several submodules:
//!
//! - [`node`]: Defines the internal structure of AST nodes.
//! - [`kind`]: Enumerates the kinds of AST nodes (e.g., keyword, identifier).
//! - [`content`]: Defines the data payload associated with AST nodes.
//! - [`iterators`]: Contains pre-order and post-order traversal iter.
//!
//! # Re-exports
//!
//! To simplify access to key types, several items are re-exported:
//!
//! - [`AstNode`] — alias of `syntax::Node`
//! - [`AstKind`] — alias of `kind::Kind`
//! - [`AstContent`] — alias of `content::Content`
//! - [`Ast`] — the full parsed AST structure
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::syntax::{Ast, AstNode, AstKind};
//!
//! let ast: Ast = /* parse something */;
//! let root: &AstNode = ast.root();
//!
//! println!("Root kind: {:?}", root.kind());
//! ```
//!
//! # Traversal
//!
//! Use the iter in [`iterators`] to walk through the AST:
//!
//! ```rust
//! use aiplan4rust::syntax::iter::PreorderIter;
//!
//! let iter = PreorderIter::new(ast.root());
//! for (syntax, depth) in iter {
//!     println!("{}- {:?}", "  ".repeat(depth), syntax.kind());
//! }
//! ```

// Submodules
pub mod content;
pub mod kind;

pub mod from_ast;
pub mod ast;

pub mod node;
pub mod renderer;
pub mod validator;

// Public re-exports
pub use kind::Kind as AstKind;
pub use content::Content as AstContent;
pub use from_ast::FromAst;
pub use ast::Ast;
pub use node::AstNode;

pub use renderer::syntax::syntax::render_with_indent;

pub use validator::validate;
