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
//! - [`SyntaxNode`]: the core trait representing a syntax node, including
//!   management of node kind and content types.
//! - Support for formatted display with indentation and identifier resolution.
//! - Methods for extracting and transforming syntactic content.
//!
//! # Usage
//!
//! Implementations of the [`SyntaxNode`] trait enable integration of
//! specific languages into the platform, providing required behaviors
//! for display, manipulation, and parsing.
//!
//! # Example
//!
//! ```rust
//! let syntax: MySyntaxNode = ...;
//! println!("{}", syntax);
//! ```

use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use ordered_float::OrderedFloat;

use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, Ident, Optimization};
use crate::aiplan4rust::semantic::symbol::Symbol;
use crate::aiplan4rust::syntax::tree::{SyntaxContent, SyntaxTree};
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::tree::renderers::kind::Kind;
use crate::aiplan4rust::syntax::tree::renderers::RenderKind;

/// Trait representing a node in a syntax tree.
///
/// This trait extends [`ArenaNode`] and [`Display`], providing methods to
/// access the node’s kind and semantic content, as well as formatting and
/// extracting various information.
///
/// # Associated Types
///
/// - `Kind`: the type representing the category or kind of the syntax node.
/// - `Content`: the type representing the semantic content attached to the node.
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
///     type Kind = MyKind;
///     type Content = MyContent;
///
///     fn kind(&self) -> Self::Kind { /* ... */ }
///     fn set_kind(&mut self, kind: Self::Kind) { /* ... */ }
///     fn content(&self) -> &Self::Content { /* ... */ }
///     fn content_mut(&mut self) -> &mut Self::Content { /* ... */ }
///     // other methods ...
/// }
/// ```
pub trait SyntaxNode: ArenaNode + Display {
    /// The type used to represent the syntax's kind.
    ///
    /// Must implement `Copy`, `Debug`, and `Display` traits.
    type Kind: Copy + Debug + Display;

    /// The type used to represent the semantic content of the syntax.
    ///
    /// Must implement the `SyntaxContent` trait.
    type Content: SyntaxContent;

    /// Returns the kind of the syntax.
    ///
    /// # Returns
    ///
    /// The current kind of the syntax node, of associated type `Kind`.
    fn kind(&self) -> Self::Kind;

    /// Sets the kind of the syntax.
    ///
    /// # Arguments
    ///
    /// * `kind` - The new kind to assign to the syntax node.
    fn set_kind(&mut self, kind: Self::Kind);

    /// Returns the rendering kind of the node.
    ///
    /// This method provides a `RenderKind` derived from the node's `Kind`.
    /// It is used by the rendering engine to determine which formatting
    /// rules to apply, for example for PDDL/HDDL output or tree visualization.
    ///
    /// # Returns
    ///
    /// A `RenderKind` corresponding to the node's concrete type for rendering.
    fn render_kind(&self) -> RenderKind;

    /// Returns a reference to the syntax's semantic content.
    ///
    /// # Returns
    ///
    /// A reference to the content of the syntax node, of associated type `Content`.
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

    /// Returns the identifier if present in the syntax’s content.
    ///
    /// # Returns
    ///
    /// An `Option<Ident>` containing the identifier if it exists, or `None` otherwise.
    fn as_ident(&self) -> Option<Ident> {
        self.content().as_ident()
    }

    /// Returns the floating-point literal if present in the syntax’s content.
    ///
    /// # Returns
    ///
    /// An `Option<OrderedFloat<f64>>` containing the float literal if it exists, or `None` otherwise.
    fn as_float(&self) -> Option<OrderedFloat<f64>> {
        self.content().as_float()
    }

    /// Returns the binary comparison operator if present in the syntax’s content.
    ///
    /// # Returns
    ///
    /// An `Option<BinaryComp>` containing the operator if it exists, or `None` otherwise.
    fn as_binary_comp(&self) -> Option<BinaryComp> {
        self.content().as_binary_comp()
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
    fn as_optimization(&self) -> Option<Optimization> {
        self.content().as_optimization()
    }

    /// Returns the symbol reference if present in the syntax’s content.
    ///
    /// # Returns
    ///
    /// A `Result<Option<Symbol>, SyntaxTreeError>` containing the symbol reference
    /// if it exists, or an error if the extraction failed.
    fn as_symbol(&self) -> Result<Option<Symbol>, SyntaxTreeError>;

    /// Attempts to extract an identifier from the syntax’s content.
    ///
    /// # Returns
    ///
    /// A `Result<Ident, SyntaxTreeError>` containing the identifier if successful,
    /// or an error if extraction failed.
    fn try_ident(&self) -> Result<Ident, SyntaxTreeError> {
        self.content().try_ident()
    }

    /// Attempts to extract a floating-point literal from the syntax’s content.
    ///
    /// # Returns
    ///
    /// A `Result<OrderedFloat<f64>, SyntaxTreeError>` containing the float if successful,
    /// or an error if extraction failed.
    fn try_float(&self) -> Result<OrderedFloat<f64>, SyntaxTreeError> {
        self.content().try_float()
    }

    /// Attempts to extract a binary comparison operator from the syntax’s content.
    ///
    /// # Returns
    ///
    /// A `Result<BinaryComp, SyntaxTreeError>` containing the operator if successful,
    /// or an error if extraction failed.
    fn try_binary_comp(&self) -> Result<BinaryComp, SyntaxTreeError> {
        self.content().try_binary_comp()
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
    fn try_optimization(&self) -> Result<Optimization, SyntaxTreeError> {
        self.content().try_optimization()
    }

    /// Attempts to extract a symbol  from the syntax’s content.
    ///
    /// # Returns
    ///
    /// A `Result<Symbol, SyntaxTreeError>` containing the symbol if successful,
    /// or an error if extraction failed or no symbol is present.
    fn try_symbol(&self) -> Result<Symbol, SyntaxTreeError> {
        self.as_symbol()?.ok_or_else(|| SyntaxTreeError::not_a_symbol_ref())
    }

    /// Applies identifier remapping to the content of the syntax using the provided map.
    ///
    /// # Arguments
    ///
    /// * `map` - A reference to a `HashMap` mapping old identifiers to new identifiers.
    ///
    /// # Effects
    ///
    /// Updates the syntax content by remapping identifiers according to `map`.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        self.content_mut().remap_idents(map);
    }

    /// Creates a shallow clone of the node.
    ///
    /// This method clones the node itself, including its kind and content,
    /// but **does not clone its children**. The resulting node has an empty
    /// children list and can be inserted into a tree independently.
    ///
    /// This function is primarily used by [`SyntaxTree::clone_subtree`]
    /// to clone individual nodes while recursively reconstructing a subtree.
    ///
    /// # Returns
    /// A new instance of the same node type with the same kind and content.
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

    /// Formats the syntax node with access to the entire syntax tree and an interner.
    ///
    /// This method enables the formatter to access related nodes in the syntax tree,
    /// which is useful for displaying child nodes or other contextual information.
    /// It also resolves identifiers using the provided interner.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write output to.
    /// * `syntax_tree` - Reference to the [`SyntaxTree`] containing all nodes.
    /// * `interner` - Reference to the [`StringInterner`] used for resolving identifiers.
    ///
    /// # Errors
    ///
    /// Returns a [`fmt::Result::Err`] if writing to the formatter fails.
    fn fmt_with_interner(
        &self,
        f: &mut Formatter<'_>,
        syntax_tree: &SyntaxTree<Self>,
        interner: &StringInterner
    ) -> fmt::Result
    where
        Self: Sized,
        <Self as SyntaxNode>::Content: SyntaxContent;


    /// Converts the syntax node to a `String` using its `fmt_with_interner` method.
    ///
    /// Wraps the node in a temporary `Display` implementation that
    /// formats the node with identifier resolution support.
    ///
    /// # Arguments
    ///
    /// * `arena` - Reference to the [`SyntaxTree`] containing the syntax nodes.
    /// * `interner` - Reference to the [`StringInterner`] used to resolve identifiers.
    ///
    /// # Returns
    ///
    /// A `String` representing the formatted syntax with interner-based resolution.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = syntax.to_string_with_interner(&syntax_tree, &interner);
    /// println!("{}", s);
    /// ```
    fn to_string_with_interner(&self, arena: &SyntaxTree<Self>, interner: &StringInterner) -> String
    where
        Self: Sized,
        <Self as SyntaxNode>::Content: SyntaxContent,
    {
        /// Helper struct wrapping a node to implement `Display` using interner-aware formatting.
        struct DisplayWrapper<'a, T: SyntaxNode>
        where
            T: SyntaxNode,
            T::Content: SyntaxContent,
        {
            /// Reference to the syntax node being formatted.
            node: &'a T,

            /// Reference to the syntax tree (arena) holding the node.
            syntax_tree: &'a SyntaxTree<T>,

            /// Reference to the interner used for resolving identifiers.
            interner: &'a StringInterner,
        }

        impl<'a, T: SyntaxNode> fmt::Display for DisplayWrapper<'a, T>
        where
            T: SyntaxNode,
            T::Content: SyntaxContent,
        {
            /// Formats the syntax node using the provided formatter,
            /// with interner support to resolve identifiers.
            ///
            /// # Arguments
            ///
            /// * `f` - The formatter to write output to.
            ///
            /// # Returns
            ///
            /// A [`fmt::Result`] indicating success or failure.
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.node.fmt_with_interner(f, self.syntax_tree, self.interner)
            }
        }

        format!("{}", DisplayWrapper { node: self, syntax_tree: arena, interner })
    }

    /// Returns the number of characters used per indentation level.
    ///
    /// # Returns
    ///
    /// A `usize` representing the number of spaces (or characters) per indent level.
    ///
    /// # Example
    ///
    /// ```rust
    /// let width = SyntaxNode::indent_width();
    /// assert_eq!(width, 2);
    /// ```
    fn indent_width() -> usize {
        2
    }

    /// Returns the character used for indentation.
    ///
    /// # Returns
    ///
    /// A `char` representing the indentation character (usually a space or tab).
    ///
    /// # Example
    ///
    /// ```rust
    /// let ch = SyntaxNode::indent_char();
    /// assert_eq!(ch, ' ');
    /// ```
    fn indent_char() -> char {
        ' '
    }

    /// Constructs an indentation string for a given indentation level.
    ///
    /// # Arguments
    ///
    /// * `level` - The indentation level (number of indent units).
    ///
    /// # Returns
    ///
    /// A `String` consisting of the indentation character repeated
    /// `level * indent_width()` times.
    ///
    /// # Example
    ///
    /// ```rust
    /// let indent = SyntaxNode::make_indent(3);
    /// assert_eq!(indent, "      "); // 3 levels * 2 spaces each = 6 spaces
    /// ```
    fn make_indent(level: usize) -> String {
        let total = level * Self::indent_width();
        std::iter::repeat(Self::indent_char())
            .take(total)
            .collect()
    }

    /// Formats the syntax using a specific syntax syntax style,
    /// applying the given indentation level.
    ///
    /// This method is similar to `fmt_with`, but formats the syntax
    /// according to custom grammar or syntax conventions (e.g., PDDL-like syntax).
    /// Implementors should respect the provided `indent` level to properly
    /// indent the output.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the formatted output to.
    /// * `syntax_tree` - A reference to the [`SyntaxTree`] containing all syntax nodes.
    /// * `interner` - A reference to the [`StringInterner`] used to resolve identifiers.
    /// * `indent` - The indentation level (number of indent units to apply).
    ///
    /// # Errors
    ///
    /// Returns a [`fmt::Error`] if writing to the formatter fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// syntax.fmt_syntax_with_indent(&mut formatter, &syntax_tree, &interner, 2)?;
    /// ```
    fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        syntax_tree: &SyntaxTree<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result
    where
        Self: Sized,
        <Self as SyntaxNode>::Content: SyntaxContent;

    /// Formats the syntax using a specific syntax syntax style without indentation.
    ///
    /// This is a convenience method that delegates to
    /// [`fmt_syntax_with_indent`] with an indent level of 0.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the formatted output to.
    /// * `syntax_tree` - A reference to the [`SyntaxTree`] containing all syntax nodes.
    /// * `interner` - A reference to the [`StringInterner`] used to resolve identifiers.
    ///
    /// # Errors
    ///
    /// Returns a [`fmt::Error`] if writing to the formatter fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// syntax.fmt_syntax(&mut formatter, &syntax_tree, &interner)?;
    /// ```
    fn fmt_syntax(
        &self,
        f: &mut Formatter<'_>,
        syntax_tree: &SyntaxTree<Self>,
        interner: &StringInterner,
    ) -> fmt::Result
    where
        Self: Sized,
        <Self as SyntaxNode>::Content: SyntaxContent,
    {
        self.fmt_syntax_with_indent(f, syntax_tree, interner, 0)
    }


    /// Converts the syntax node to a string using a specific syntax format,
    /// applying the given indentation level.
    ///
    /// This method wraps the syntax node in a temporary formatter struct,
    /// which produces a syntax-oriented string representation by delegating
    /// the formatting to the node's `fmt_syntax_with_indent` method.
    ///
    /// # Arguments
    ///
    /// * `syntax_tree` - A reference to the [`SyntaxTree`] containing the syntax nodes.
    /// * `interner` - A reference to the [`StringInterner`] used to resolve identifier strings.
    /// * `indent` - The indentation level (number of indent units to apply).
    ///
    /// # Returns
    ///
    /// A `String` containing the formatted syntax representation of the node,
    /// with indentation applied.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = syntax.to_syntax_with_indent(&syntax_tree, &interner, 2);
    /// println!("{}", s);
    /// ```
    fn to_syntax_with_indent(
        &self,
        syntax_tree: &SyntaxTree<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> String
    where
        Self: Sized,
        <Self as SyntaxNode>::Content: SyntaxContent,
    {
        /// A helper struct to implement `Display` for a syntax node with context.
        ///
        /// Holds references to the node itself, the containing syntax tree,
        /// the interner for identifier resolution, and the current indentation level.
        struct SyntaxDisplayWrapper<'a, T: SyntaxNode>
        where
            T: SyntaxNode,
            T::Content: SyntaxContent,
        {
            /// The syntax node to format.
            node: &'a T,

            /// Reference to the syntax tree containing the node.
            syntax_tree: &'a SyntaxTree<T>,

            /// Reference to the interner used to resolve identifiers.
            interner: &'a StringInterner,

            /// The indentation level to apply.
            indent: usize,
        }

        impl<'a, T: SyntaxNode> fmt::Display for SyntaxDisplayWrapper<'a, T>
        where
            T: SyntaxNode,
            T::Content: SyntaxContent,
        {
            /// Formats the syntax node using the given indentation and interner,
            /// delegating to the node's `fmt_syntax_with_indent` method.
            ///
            /// # Arguments
            ///
            /// * `f` - The formatter used to write the output.
            ///
            /// # Returns
            ///
            /// A `fmt::Result` indicating success or failure.
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
            where
                T: SyntaxNode,
                T::Content: SyntaxContent,
            {
                self.node
                    .fmt_syntax_with_indent(f, self.syntax_tree, self.interner, self.indent)
            }
        }

        format!(
            "{}",
            SyntaxDisplayWrapper {
                node: self,
                syntax_tree,
                interner,
                indent,
            }
        )
    }

    /// Converts the syntax node to a string using the specific syntax formatting
    /// without any indentation.
    ///
    /// This method is a convenience wrapper around
    /// `to_syntax_with_indent`, calling it with an indentation level of 0.
    ///
    /// # Arguments
    ///
    /// * `syntax_tree` - A reference to the [`SyntaxTree`] containing the syntax nodes.
    /// * `interner` - A reference to the [`StringInterner`] used to resolve identifier strings.
    ///
    /// # Returns
    ///
    /// A `String` containing the formatted syntax representation of the node
    /// without indentation.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = syntax.to_syntax_string(&arena, &interner);
    /// println!("{}", s);
    /// ```
    fn to_syntax_string(
        &self,
        syntax_tree: &SyntaxTree<Self>,
        interner: &StringInterner,
    ) -> String
    where
        Self: Sized,
        <Self as SyntaxNode>::Content: SyntaxContent,
    {
        self.to_syntax_with_indent(syntax_tree, interner, 0)
    }
}
