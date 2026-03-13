//! Syntax module
//!
//! This module defines the fundamental traits and structures for representing
//! and manipulating abstract syntax of supported languages.
//!
//! It relies on a tree-based representation (`SyntaxTree`) and an arena system
//! for efficient management of syntax nodes. The module also facilitates
//! identifier management via an interner and the handling of semantic content
//! associated with nodes.
//!
//! # Key Components
//!
//! - [`Node`]: the common trait representing a syntax node, including
//!   management of node kind and content types.
//! - Support for formatted display with indentation and identifier resolution.
//! - Methods for extracting and transforming syntactic content.
//!
//! # Usage
//!
//! Implementations of the [`Node`] trait enable integration of
//! specific languages into the platform, providing required behaviors
//! for display, manipulation, and parsing.
//!
//! # Example
//!
//! ```rust
//! let syntax: MySyntaxNode = ...;
//! println!("{}", syntax);
//! ```


use std::fmt::{Debug, Display};
use ordered_float::OrderedFloat;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, CompareOp, OptimizationOp};
use crate::aiplan4rust::tree::SyntaxContent;
use crate::aiplan4rust::tree::error::SyntaxTreeError;

/// Trait representing a node in a syntax tree.
///
/// This trait extends [`ArenaNode`] and [`Display`], providing methods to
/// access the node’s kind and semantic content, as well as formatting and
/// extracting various information.
///
/// # Associated Types
///
/// - `Kind`: the either_type representing the category or kind of the syntax node.
/// - `Content`: the either_type representing the semantic content attached to the node.
///
/// # Core Features
///
/// - Access and modification of the node’s kind.
/// - Access to semantic content (immutable and mutable).
/// - Extraction of specific data (identifiers, operators, literals).
/// - Custom formatting with support for indentation and interner resolution.
/// - Identifier remapping within the content.
///
/// # Example Implementation
///
/// ```rust
/// impl SyntaxNode for MySyntaxNode {
///     either_type Kind = MyKind;
///     either_type Content = MyContent;
///
///     fn kind(&self) -> Self::Kind { /* ... */ }
///     fn set_kind(&mut self, kind: Self::Kind) { /* ... */ }
///     fn content(&self) -> &Self::Content { /* ... */ }
///     fn content_mut(&mut self) -> &mut Self::Content { /* ... */ }
///     // other methods ...
/// }
/// ```
pub trait Node: ArenaNode + Display {
    /// The either_type used to represent the syntax's kind.
    ///
    /// Must implement `Copy`, `Debug`, and `Display` traits.
    type Kind: Copy + Debug + Display;

    /// The either_type used to represent the semantic content of the syntax.
    ///
    /// Must implement the `SyntaxContent` trait.
    type Content: SyntaxContent;

    /// Returns the kind of the syntax.
    ///
    /// # Returns
    ///
    /// The current kind of the syntax node, of associated either_type `Kind`.
    fn kind(&self) -> Self::Kind;

    /// Sets the kind of the syntax.
    ///
    /// # Arguments
    ///
    /// * `kind` - The new kind to assign to the syntax node.
    fn set_kind(&mut self, kind: Self::Kind);

    /// Returns a reference to the syntax's semantic content.
    ///
    /// # Returns
    ///
    /// A reference to the content of the syntax node, of associated either_type `Content`.
    fn content(&self) -> &Self::Content;

    /// Returns a mutable reference to the syntax's semantic content.
    ///
    /// # Returns
    ///
    /// A mutable reference to the content of the syntax node, allowing modification.
    fn content_mut(&mut self) -> &mut Self::Content;

    /// Sets the semantic content of the syntax node.
    ///
    /// This function replaces the current content of the node with the provided value.
    /// It takes ownership of `new_content` and overwrites any existing content.
    ///
    /// # Parameters
    /// - `new_content`: The new content to assign to this node.
    ///
    /// # Notes
    /// - This is a direct replacement; any previous content will be dropped.
    /// - Use `content_mut()` if you only need to modify the existing content without
    ///   replacing it entirely.
    ///
    /// # Example
    /// ```ignore
    /// let mut node = tree.try_node_mut(node_id)?;
    /// node.set_content(Content::Number(42.0));
    /// ```
    fn set_content(&mut self, new_content: Self::Content);

    /// Checks whether the syntax node has no meaningful content.
    ///
    /// # Returns
    ///
    /// `true` if the syntax's content is considered empty or none; `false` otherwise.
    ///
    /// # Default behavior
    ///
    /// By default, this delegates to `content().is_none()`.
    fn has_content(&self) -> bool {
        self.content().is_none()
    }

    // Delegation methods to the syntax’s content, allowing convenient extraction
    // of specific semantic types without manually matching on content.

    /// Returns the floating-point literal if present in the syntax’s content.
    ///
    /// # Returns
    ///
    /// An `Option<OrderedFloat<f64>>` containing the float literal if it exists, or `None` otherwise.
    fn as_number(&self) -> Option<OrderedFloat<f64>> {
        self.content().as_number()
    }

    /// Returns the binary comparison operator if present in the syntax’s content.
    ///
    /// # Returns
    ///
    /// An `Option<BinaryComp>` containing the operator if it exists, or `None` otherwise.
    fn as_compare_op(&self) -> Option<CompareOp> {
        self.content().as_compare_op()
    }

    /// Returns the assignment operator if present in the syntax’s content.
    ///
    /// # Returns
    ///
    /// An `Option<AssignOp>` containing the assignment operator if it exists, or `None` otherwise.
    fn as_assign_op(&self) -> Option<AssignOp> {
        self.content().as_assign_op()
    }

    /// Returns the arithmetic operator if present in the syntax’s content.
    ///
    /// # Returns
    ///
    /// An `Option<ArithmeticOp>` containing the arithmetic operator if it exists, or `None` otherwise.
    fn as_arithmetic_op(&self) -> Option<ArithmeticOp> {
        self.content().as_arithmetic_op()
    }

    /// Returns the optimization directive if present in the syntax’s content.
    ///
    /// # Returns
    ///
    /// An `Option<Optimization>` containing the optimization directive if it exists, or `None` otherwise.
    fn as_optimization(&self) -> Option<OptimizationOp> {
        self.content().as_optimization_op()
    }

    /// Attempts to extract a floating-point literal from the syntax’s content.
    ///
    /// # Returns
    ///
    /// A `Result<OrderedFloat<f64>, SyntaxTreeError>` containing the float if successful,
    /// or an error if extraction failed.
    fn try_number(&self) -> Result<OrderedFloat<f64>, SyntaxTreeError> {
        self.content().try_number()
    }

    /// Attempts to extract a binary comparison operator from the syntax’s content.
    ///
    /// # Returns
    ///
    /// A `Result<BinaryComp, SyntaxTreeError>` containing the operator if successful,
    /// or an error if extraction failed.
    fn try_compare_op(&self) -> Result<CompareOp, SyntaxTreeError> {
        self.content().try_compare_op()
    }

    /// Attempts to extract an assignment operator from the syntax’s content.
    ///
    /// # Returns
    ///
    /// A `Result<AssignOp, SyntaxTreeError>` containing the operator if successful,
    /// or an error if extraction failed.
    fn try_assign_op(&self) -> Result<AssignOp, SyntaxTreeError> {
        self.content().try_assign_op()
    }

    /// Attempts to extract an arithmetic operator from the syntax’s content.
    ///
    /// # Returns
    ///
    /// A `Result<ArithmeticOp, SyntaxTreeError>` containing the operator if successful,
    /// or an error if extraction failed.
    fn try_arithmetic_op(&self) -> Result<ArithmeticOp, SyntaxTreeError> {
        self.content().try_arithmetic_op()
    }

    /// Attempts to extract an optimization directive from the syntax’s content.
    ///
    /// # Returns
    ///
    /// A `Result<Optimization, SyntaxTreeError>` containing the directive if successful,
    /// or an error if extraction failed.
    fn try_optimization(&self) -> Result<OptimizationOp, SyntaxTreeError> {
        self.content().try_optimization_op()
    }

    /// Creates a shallow clone of the node.
    ///
    /// This method clones the node itself, including its kind and content,
    /// but **does not clone its children**. The resulting node has an empty
    /// children list and can be inserted into a tree independently.
    ///
    /// This function is primarily used by [`Tree::clone_subtree`]
    /// to clone individual nodes while recursively reconstructing a subtree.
    ///
    /// # Returns
    /// A new instance of the same node either_type with the same kind and content.
    ///
    /// # Example
    /// ```ignore
    /// let original: ExprNode = ...;
    /// let clone = original.clone_shallow();
    /// assert_eq!(clone.kind(), original.kind());
    /// assert_eq!(clone.content(), original.content());
    /// assert!(clone.children().is_empty());
    /// ```
    fn clone_shallow(&self) -> Self;

    /// Returns `true` if the node represents an **atomic formula**.
    ///
    /// For `ExprNode`, this typically means `ExprKind::AtomicFormula` or `ExprKind::FComp`.
    /// This is used to identify literals in logical expr.
    fn is_atomic_formula(&self) -> bool;

    /// Returns `true` if the node is a **temporal specifier**.
    ///
    /// Temporal specifiers are nodes like `AtStart`, `AtEnd`, or `Overall` in a PDDL expression.
    /// This is used for consistency verification and expr of temporal expr.
    fn is_time_specifier(&self) -> bool;

    /// Returns `true` if the node represents a **logical operator**.
    ///
    /// Logical operators typically include `And`, `Or`, and `Not`. This is useful for
    /// traversals, expr, or propagation of temporal specifiers through logical nodes.
    fn is_logic(&self) -> bool;

    /// Returns `true` if this node represents a logical negation (`Not`).
    ///
    /// This enables generic algorithms (such as literal detection)
    /// to operate independently of the specific `Kind` enum used.
    fn is_not(&self) -> bool;

    /// Returns `true` if this node represents a logical variable.
    ///
    /// This is used during atom extraction to distinguish between parameters
    /// of an action and constant objects.
    fn is_variable(&self) -> bool;
}
