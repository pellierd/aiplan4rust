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
use crate::aiplan4rust::semantic::symbol::{SymbolKind, Symbol};
use crate::aiplan4rust::syntax::tree::NodeId;
use crate::aiplan4rust::core::arena::ArenaNode;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};
use crate::aiplan4rust::syntax;
use crate::aiplan4rust::syntax::tree::{SyntaxBaseNode, SyntaxNode, SyntaxTree};
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::tree::renderers::{RenderKind};

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
    pub fn new(kind: ExprKind, content: ExprContent, parent: Option<NodeId>) -> Self {
        ExprNode {
            inner: SyntaxBaseNode::new(kind, content, Vec::new(), parent),
        }
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
        syntax::tree::renderers::default_rendering(self, f)
    }
}

/// Implements the `ArenaNode` trait enabling arena-based tree management.
///
/// Provides methods to get/set parent, children, and add a child node.
impl ArenaNode for ExprNode {
    fn parent(&self) -> Option<NodeId> {
        self.inner.parent()
    }

    fn set_parent(&mut self, parent: Option<NodeId>) {
        self.inner.set_parent(parent)
    }

    fn children(&self) -> &[NodeId] {
        self.inner.children()
    }

    fn set_children(&mut self, children: Vec<NodeId>) {
        self.inner.set_children(children)
    }

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
impl SyntaxNode for ExprNode {
    type Kind = ExprKind;
    type Content = ExprContent;

    fn kind(&self) -> Self::Kind {
        self.inner.kind()
    }

    fn set_kind(&mut self, kind: Self::Kind) {
        self.inner.set_kind(kind);
    }

    /// Returns the rendering kind of this Expr node.
    ///
    /// The `render_kind` provides a high-level categorization of the node
    /// that is used by renderers to determine how to display it. This
    /// abstracts over the specific underlying `ExprNode` and maps it to a `RenderKind` variant.
    ///
    /// # Returns
    /// A `RenderKind` value representing the node’s appearance in rendered
    /// output. This is typically used by syntax tree renderers or formatters
    /// to decide keywords, indentation, or visual representation.
    fn render_kind(&self) -> RenderKind {
        RenderKind::from_expr_kind(self.kind())
    }

    fn content(&self) -> &Self::Content {
        &self.inner.content()
    }

    fn content_mut(&mut self) -> &mut Self::Content {
        self.inner.content_mut()
    }

    /// Attempts to interpret this node as a symbol if its kind corresponds to
    /// a symbol type_checker. Returns `None` if not applicable.
    ///
    /// # Errors
    ///
    /// Returns an error if identifier extraction fails.
    fn as_symbol(&self) -> Result<Option<Symbol>, SyntaxTreeError> {
        let kind = self.kind();
        let symbol_kind = match kind {
            ExprKind::PrimitiveType => SymbolKind::PrimitiveType,
            ExprKind::Constant => SymbolKind::Constant,
            ExprKind::Variable => SymbolKind::Variable,
            ExprKind::FunctionSymbol => SymbolKind::Function,
            ExprKind::Predicate => SymbolKind::Predicate,
            ExprKind::TaskSymbol => SymbolKind::Task,
            ExprKind::TaskID => SymbolKind::TaskID,
            _ => return Ok(None),
        };

        let ident = self.try_ident()?;
        Ok(Some(Symbol::new(ident, symbol_kind)))
    }

    /// Recursively pretty-prints the syntax subtree rooted at this node,
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
    }
}
