//! Module defining the `ExprNode` struct representing nodes in the expression syntax tree
//! for the AIPlan4Rust intermediate representation (LIR).
//!
//! # Overview
//!
//! `ExprNode` is a wrapper around a generic syntax tree node (`SyntaxBaseNode`) specialized
//! for expressions, with `ExprKind` as the node kind and `ExprContent` as its content.
//! This struct supports parent-child relationships and integrates with the arena allocator model
//! through the `ArenaNode` trait, enabling efficient tree manipulation.
//!
//! # Key Features
//!
//! - Construction of expression nodes with kind, content, and optional parent.
//! - Deref coercions to access underlying `SyntaxBaseNode` functionality transparently.
//! - Display formatting for easy debugging and visualization of node properties,
//!   including kind, content, parent, and children.
//! - Integration with the `ArenaNode` trait for generic tree arena management,
//!   providing parent and children getter/setters and child addition.
//! - Implementation of the `SyntaxNode` trait providing accessors and mutators for kind
//!   and content, as well as symbol resolution when applicable.
//! - Recursive pretty-printing of the syntax subtree with interner support to display
//!   interned strings in a readable way.
//!
//! # Usage Example
//!
//! ```rust
//! use crate::aiplan4rust::lir::expr::ExprNode;
//! use crate::aiplan4rust::lir::expr::{ExprKind, ExprContent};
//!
//! let node = ExprNode::new(ExprKind::Variable, ExprContent::None, None);
//! println!("{}", node);
//! ```
//!
//! # Error Handling
//!
//! The symbol resolution method returns `Result` to handle cases where
//! identification extraction fails or when the node kind does not correspond to a symbol.

use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lir::expr::{ExprContent, ExprKind};
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::arena::ArenaNode;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::tree::{SyntaxBaseNode, Node, Tree};

/// Expression node wrapping a syntax base node specialized with `ExprKind` and `ExprContent`.
///
/// This struct represents a node in the expression syntax tree with
/// hierarchical parent-child relationships managed via node IDs.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ExprNode {
    inner: SyntaxBaseNode<ExprKind, ExprContent>,
}

impl ExprNode {
    /// Creates a new `ExprNode` with the specified kind, content, and optional parent.
    ///
    /// # Parameters
    ///
    /// - `kind`: The kind of expression node (e.g., Variable, Constant, Function).
    /// - `content`: Additional content associated with the node (e.g., identifiers, literals).
    /// - `parent`: Optional parent node ID, if this node is a child in a syntax tree.
    ///
    /// # Returns
    ///
    /// A new `ExprNode` instance with empty children.
    pub fn new(kind: ExprKind, content: Content, parent: Option<NodeId>) -> Self {
        ExprNode {
            inner: SyntaxBaseNode::new(kind, content, Vec::new(), parent),
        }
    }

    /// Checks if the expression is an `And` expression with no children.
    ///
    /// Returns `true` if `self` is of kind `ExprKind::And` and has no children,
    /// otherwise returns `false`.
    pub fn is_empty_and(&self) -> bool {
        self.kind() == ExprKind::And && self.children().is_empty()
    }

    /// Checks if the expression is an `Or` expression with no children.
    ///
    /// Returns `true` if `self` is of kind `ExprKind::Or` and has no children,
    /// otherwise returns `false`.
    pub fn is_empty_or(&self) -> bool {
        self.kind() == ExprKind::Or && self.children().is_empty()
    }

    pub fn to_syntax_string(
        &self,
        tree: &Tree<ExprNode>,
        interner: &StringInterner
    ) -> String {
        struct Wrapper<'a>(&'a ExprNode, &'a Tree<ExprNode>, &'a StringInterner);
        impl fmt::Display for Wrapper<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                // Appelle ici le renderer spécifique au LIR/Expr
                // Si tu n'en as pas encore, utilise la version générique ou simplifiée
               writeln!(f, "{}", "TO DO".to_string())
            }
        }
        format!("{}", Wrapper(self, tree, interner))
    }


}

/// Allows transparent access to the underlying `SyntaxBaseNode` via dereferencing.
impl Deref for ExprNode {
    type Target = SyntaxBaseNode<ExprKind, ExprContent>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Allows mutable access to the underlying `SyntaxBaseNode` via dereferencing.
impl DerefMut for ExprNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Implements the `Display` trait to format an `ExprNode` as a string, showing:
/// - The node's kind
/// - The node's content
/// - The parent node ID (or "none" if absent)
/// - A list of children node IDs
impl fmt::Display for ExprNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", "TO DO".to_string())
    }
}

/// Implements the `ArenaNode` trait enabling arena-based tree management.
///
/// Provides methods to get/set parent, children, and add a child node.
impl ArenaNode for ExprNode {
    /// Returns the parent node ID of this node, if any.
    ///
    /// # Returns
    ///
    /// An `Option<NodeId>` containing the parent node ID, or `None` if this node has no parent.
    fn parent(&self) -> Option<NodeId> {
        self.inner.parent()
    }

    /// Sets the parent node ID of this node.
    ///
    /// # Arguments
    ///
    /// * `parent` - An `Option<NodeId>` representing the new parent node. Use `None` to remove the parent.
    fn set_parent(&mut self, parent: Option<NodeId>) {
        self.inner.set_parent(parent)
    }

    /// Returns a slice of child node IDs.
    ///
    /// # Returns
    ///
    /// A slice (`&[NodeId]`) containing all direct children of this node.
    fn children(&self) -> &[NodeId] {
        self.inner.children()
    }

    /// Returns a mutable slice of child node IDs.
    ///
    /// # Returns
    ///
    /// A mutable slice (`&mut Vec<NodeId>`) containing all direct children of this node,
    /// allowing modification of the children.
    fn children_mut(&mut self) -> &mut Vec<NodeId> {
        self.inner.children_mut()
    }

    /// Replaces all child node IDs of this node.
    ///
    /// # Arguments
    ///
    /// * `children` - A `Vec<NodeId>` containing the new list of children.
    fn set_children(&mut self, children: Vec<NodeId>) {
        self.inner.set_children(children)
    }

    /// Adds a child node ID to this node.
    ///
    /// # Arguments
    ///
    /// * `child` - The `NodeId` of the child node to add.
    fn add_child(&mut self, child: NodeId) {
        self.inner.add_child(child)
    }
}

/// Implements the `SyntaxNode` trait for `ExprNode`,
/// which provides accessors for the node's kind and content,
/// as well as functionality for symbol resolution and formatted output.
///
/// # Symbol Resolution
///
/// The `as_symbol_ref` method returns a `SymbolRef` if the node's kind corresponds to a symbol,
/// such as a variable, constant, or function. Otherwise, it returns `None`.
impl Node for ExprNode {
    type Kind = ExprKind;
    type Content = ExprContent;

    /// Returns the kind of the node.
    ///
    /// The kind represents the type of expression the node encodes,
    /// for example `AtomicFormula`, `FComp`, `And`, `Or`, `AtStart`, etc.
    ///
    /// # Returns
    ///
    /// The current kind of the node, of type `ExprKind`.
    ///
    /// # Example
    /// ```
    /// if node.kind() == ExprKind::AtomicFormula {
    ///     println!("This node is an atomic formula");
    /// }
    /// ```
    fn kind(&self) -> Self::Kind {
        self.inner.kind()
    }

    /// Sets the kind of the node.
    ///
    /// This replaces the current kind of the node with the specified one.
    /// It does not modify the node's children or content; it only changes the type of the node.
    ///
    /// # Arguments
    ///
    /// * `kind` - The new `ExprKind` to assign to this node.
    ///
    /// # Example
    /// ```
    /// node.set_kind(ExprKind::AtStart);
    /// ```
    fn set_kind(&mut self, kind: Self::Kind) {
        self.inner.set_kind(kind);
    }

    /// Returns a reference to the semantic content of the node.
    ///
    /// This allows read-only access to the data associated with the node,
    /// such as arguments of an atomic formula, values of a fluent comparison,
    /// or other metadata stored in `ExprContent`.
    ///
    /// # Returns
    ///
    /// A reference to the node's content.
    ///
    /// # Example
    /// ```
    /// let content_ref = node.content();
    /// ```
    fn content(&self) -> &Self::Content {
        &self.inner.content()
    }

    /// Returns a mutable reference to the semantic content of the node.
    ///
    /// This allows modifying the data associated with the node, e.g., changing
    /// arguments of an atomic formula, updating a fluent comparison, or
    /// adjusting other metadata.
    ///
    /// # Returns
    ///
    /// A mutable reference to the node's content.
    ///
    /// # Example
    /// ```
    /// node.content_mut().modify_something();
    /// ```
    fn content_mut(&mut self) -> &mut Self::Content {
        self.inner.content_mut()
    }

    /// Replaces the content of this AST node.
    ///
    /// # Arguments
    /// * `content` - The new content to assign.
    fn set_content(&mut self, content: Content) {
        self.inner.set_content(content);
    }

    /// Creates a shallow clone of the expression node.
    ///
    /// This clone copies the node's kind and content, but **does not include**
    /// its parent or children. The resulting node has an empty children list and no parent.
    ///
    /// This is typically used when reconstructing a subtree in-place within
    /// a [`Tree`] or [`Expr`] using methods like [`Tree::clone_subtree`]
    /// or `Expr`’s equivalent subtree-cloning functions.
    ///
    /// # Returns
    /// A new `ExprNode` with the same kind and content, but without parent or children.
    ///
    /// # Example
    /// ```ignore
    /// let node: ExprNode = ...;
    /// let shallow = node.clone_shallow();
    /// assert_eq!(shallow.kind(), node.kind());
    /// assert_eq!(shallow.content(), node.content());
    /// assert!(shallow.children().is_empty());
    /// assert!(shallow.parent().is_none());
    /// ```
    fn clone_shallow(&self) -> Self {
        ExprNode {
            inner: SyntaxBaseNode::new(
                self.inner.kind(),
                self.inner.content().clone(),
                Vec::new(),         // no children
                None,               // no parent
            ),
        }
    }

    /// Returns `true` if the node represents an **atomic formula**.
    ///
    /// In the expression tree, this typically corresponds to either:
    /// - `AtomicFormula`: a basic predicate or proposition.
    /// - `FComp`: a fluent comparison.
    ///
    /// # Example
    /// ```
    /// assert!(node.is_atomic_formula());
    /// ```
    fn is_atomic_formula(&self) -> bool {
        matches!(self.kind(), ExprKind::AtomicFormula | ExprKind::FComp)
    }

    /// Returns `true` if the node is a **temporal specifier**.
    ///
    /// Temporal specifiers are nodes like:
    /// - `AtStart`
    /// - `AtEnd`
    /// - `Overall`
    ///
    /// They indicate when the literal should hold in PDDL temporal expressions.
    ///
    /// # Example
    /// ```
    /// assert!(node.is_time_specifier());
    /// ```
    fn is_time_specifier(&self) -> bool {
        matches!(self.kind(), ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall)
    }

    /// Returns `true` if the node represents a **logical operator**.
    ///
    /// Logical operators include:
    /// - `And`
    /// - `Or`
    /// - `Not`
    /// - `Imply`
    ///
    /// Useful for expression traversal, normalization, and propagation of temporal specifiers.
    ///
    /// # Example
    /// ```
    /// assert!(node.is_logic());
    /// ```
    fn is_logic(&self) -> bool {
        matches!(self.kind(), ExprKind::And | ExprKind::Or | ExprKind::Not | ExprKind::Imply)
    }

    /// Returns `true` if this node represents a logical negation (`Not`).
    ///
    /// This enables generic algorithms (such as literal detection)
    /// to operate independently of the specific `Kind` enum used.
    fn is_not(&self) -> bool {
        matches!(self.kind(), ExprKind::Not)
    }

    /*/// Recursively pretty-prints the syntax subtree rooted at this node,
    /// formatting the tree structure with branch graphics and displaying interned strings.
    ///
    /// # Parameters
    ///
    /// - `f`: The formatter to write output to.
    /// - `arena`: The syntax tree arena containing nodes.
    /// - `interner`: The string interner for resolving interned identifiers.
    ///
    /// # Returns
    ///
    /// A formatting result.
    fn fmt_with_interner(
        &self,
        f: &mut Formatter<'_>,
        arena: &SyntaxTree<Self>,
        interner: &StringInterner,
    ) -> fmt::Result {
            syntax::tree::renderers::tree_rendering(self, f, arena, interner)
    }

    /// Formats the syntax subtree with indentation.
    /// This is currently a wrapper around `fmt_with_interner`.
    ///
    /// # Parameters
    ///
    /// - `f`: The formatter to write output to.
    /// - `arena`: The syntax tree arena containing nodes.
    /// - `interner`: The string interner for resolving interned identifiers.
    /// - `indent`: The number of indentation spaces (currently unused).
    ///
    /// # Returns
    ///
    /// A formatting result.
    fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        arena: &SyntaxTree<Self>,
        interner: &StringInterner,
        _indent: usize,
    ) -> fmt::Result
    where
        Self: Sized,
    {
        syntax::tree::renderers::syntax_rendering(self, f, arena, interner)
    }*/
}
