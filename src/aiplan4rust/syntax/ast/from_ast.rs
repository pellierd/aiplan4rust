//! Provides the `FromAst` trait for constructing typed representations from AST nodes.
//!
//! This module defines the [`FromAst`] trait, which is used to convert [`AstNode`] instances
//! (from the parsed abstract syntax tree) into semantic or intermediate representations.
//!
//! Implementations of this trait specify how to traverse and interpret an AST node
//! in order to produce domain-specific structures (e.g., actions, tasks, predicates).
//!
//! # Example
//! ```ignore
//! use crate::aiplan4rust::semantic::action::Action;
//! use crate::aiplan4rust::syntax::ast::AstNode;
//! use crate::aiplan4rust::arena::Arena;
//! use crate::aiplan4rust::frontend::ParserInternalError;
//!
//! impl FromAst for Action {
//!     fn from_ast(
//!         node: &AstNode,
//!         arena: &Arena<AstNode>,
//!     ) -> Result<Self, ParserInternalError> {
//!         // Parse the node and build an Action instance.
//!         Ok(Action::default())
//!     }
//! }
//! ```
//!
//! # Errors
//! Implementations typically return [`AiplanError`] if parsing fails
//! (e.g., due to missing fields, invalid identifiers, or unexpected node kinds).
//!
//! # See Also
//! - [`AstNode`] — the underlying syntax tree node.
//! - [`Arena`] — the storage structure holding all AST nodes.
//! - [`AiplanError`] — the error type used during parsing and semantic validation.

use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::core::SyntaxTree;

/// Trait for constructing an instance of a type from an AST syntax.
///
/// # Overview
/// Types implementing this trait define how to build themselves from
/// an AST syntax (`AstArenaNode`) within a given AST context (`Ast`).
///
/// This is typically used during semantic analysis or IR construction,
/// where domain-specific structures are created by traversing and interpreting the AST.
///
/// # Method
/// - `from_ast` takes:
///    - a reference to the AST syntax representing the element to construct,
///    - a reference to the full AST context, which can be used to access
///      related nodes or additional information.
///
/// # Errors
/// The method returns a `Result<Self, ParserInternalError>`,
/// allowing to propagate errors encountered during parsing or validation.
///
/// # Requirements
/// Implementors must be sized (`Self: Sized`) to allow returning `Self`.
///
/// # Example
/// ```ignore
/// impl FromAst for Action {
///     fn from_ast(syntax: &AstArenaNode, ast: &Ast) -> Result<Self, ParserInternalError> {
///         // Implementation to parse Action from AST syntax
///     }
/// }
/// ```
pub trait FromAst {
    fn from_ast(
        node: &AstNode,
        ast: &SyntaxTree<AstNode>,
    ) -> Result<Self, AiplanError>
    where
        Self: Sized;
}
