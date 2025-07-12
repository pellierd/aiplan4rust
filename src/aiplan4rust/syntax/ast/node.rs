//! Defines the `AstNode` type used to represent a node in the Abstract Syntax Tree (AST).
//!
//! Each `AstNode` stores:
//! - the kind of AST element (e.g., expression, statement, symbol),
//! - optional semantic content (e.g., identifiers, requirements),
//! - the source span it covers,
//! - parent and child relationships.
//!
//! This design is optimized for arena allocation, where nodes reference each other by `NodeId` indices rather than pointers.
//!
//! # Example
//! ```rust
//! use crate::aiplan4rust::syntax::{AstKind, Span};
//! use crate::aiplan4rust::semantic::arena::AstNode;
//!
//! let kind = AstKind::Expr;
//! let span = Span::default();
//! let node = AstNode::new(
//!     kind,
//!     AstContent::None,
//!     Vec::new(),
//!     span,
//!     None
//! );
//!
//! assert_eq!(node.kind(), AstKind::Expr);
//! assert!(node.children().is_empty());
//! ```

use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::arena::{Arena, ArenaNode, BaseNode, NodeId};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::{Ident, Requirement};
use crate::aiplan4rust::semantic::symbol::{SymbolKind, SymbolRef};
use crate::aiplan4rust::syntax::ast::{renderer, AstContent, AstKind};
use crate::aiplan4rust::syntax::Span;

/// Represents a node in the Abstract Syntax Tree (AST).
///
/// `AstNode` contains its `kind`, `content`, `span`, and structural relations (parent and children).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AstNode {
    base_node: BaseNode<AstKind, AstContent>,
    span: Span,
}

impl AstNode {
    /// Creates a new `AstNode` with the provided kind, content, children, span, and optional parent.
    ///
    /// # Parameters
    /// - `kind`: The AST element kind (e.g., `AstKind::Expr`).
    /// - `content`: The semantic payload of this node (e.g., identifier or requirement).
    /// - `children`: Indices of child nodes.
    /// - `span`: The source code location covered by this node.
    /// - `parent`: An optional parent node index.
    ///
    /// # Example
    /// ```
    /// let node = AstNode::new(
    ///     AstKind::Stmt,
    ///     AstContent::None,
    ///     vec![],
    ///     Span::new(0, 10),
    ///     None,
    /// );
    /// ```
    pub fn new(
        kind: AstKind,
        content: AstContent,
        children: Vec<NodeId>,
        span: Span,
        parent: Option<NodeId>,
    ) -> Self {
        let base_node = BaseNode::new(kind, content, children, parent);
        AstNode { base_node, span }
    }

    /// Returns an immutable reference to this node's source span.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns a mutable reference to this node's source span.
    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    /// Attempts to extract a `Requirement` from this node's content, if present.
    ///
    /// # Returns
    /// - `Some(Requirement)` if the content holds a requirement.
    /// - `None` otherwise.
    pub fn as_requirement(&self) -> Option<Requirement> {
        self.content().as_requirement()
    }

    /// Tries to retrieve the `Requirement` stored in this node's content.
    ///
    /// # Errors
    /// Returns `ParserInternalError` if this node does not contain a requirement.
    pub fn try_requirement(&self) -> Result<Requirement, ParserInternalError> {
        self.content().try_requirement()
    }
}

impl Deref for AstNode {
    type Target = BaseNode<AstKind, AstContent>;

    /// Dereferences to the underlying `BaseNode`.
    fn deref(&self) -> &Self::Target {
        &self.base_node
    }
}

impl DerefMut for AstNode {
    /// Mutably dereferences to the underlying `BaseNode`.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_node
    }
}

impl fmt::Display for AstNode {
    /// Formats this node as a string, including:
    /// - the kind,
    /// - the content,
    /// - the span,
    /// - the parent node ID,
    /// - the list of children IDs.
    ///
    /// Useful for debugging or logging.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        renderer::default::render(self, f)
    }
}

impl ArenaNode for AstNode {
    type Kind = AstKind;
    type Content = AstContent;

    /// Returns the kind of this AST node.
    fn kind(&self) -> Self::Kind {
        self.base_node.kind()
    }

    /// Updates the kind of this AST node.
    fn set_kind(&mut self, kind: Self::Kind) {
        self.base_node.set_kind(kind);
    }

    /// Returns a reference to this node's content.
    fn content(&self) -> &Self::Content {
        self.base_node.content()
    }

    /// Returns a mutable reference to this node's content.
    fn content_mut(&mut self) -> &mut Self::Content {
        self.base_node.content_mut()
    }

    /// Returns the optional parent node index.
    fn parent(&self) -> Option<NodeId> {
        self.base_node.parent()
    }

    /// Sets the parent node index.
    fn set_parent(&mut self, parent: Option<NodeId>) {
        self.base_node.set_parent(parent);
    }

    /// Returns the list of children node indices.
    fn children(&self) -> &[NodeId] {
        self.base_node.children()
    }

    /// Replaces the list of children node indices.
    fn set_children(&mut self, children: Vec<NodeId>) {
        self.base_node.set_children(children);
    }

    /// Adds a single child node index.
    fn add_child(&mut self, child: NodeId) {
        self.base_node.add_child(child);
    }

    /// Remaps identifier names in this node's content according to a mapping.
    ///
    /// This is useful when renaming symbols after interning.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        match self.content_mut() {
            AstContent::Ident(id) => {
                if let Some(&new_id) = map.get(id) {
                    *id = new_id;
                }
            }
            _ => {}
        }
    }

    /// Converts this node to a `SymbolRef` if it represents a known symbol.
    ///
    /// # Returns
    /// - `Ok(Some(SymbolRef))` if the node is a known symbol.
    /// - `Ok(None)` if it does not correspond to a symbol.
    /// - `Err(ParserInternalError)` if an identifier is missing.
    fn as_symbol_ref(&self) -> Result<Option<SymbolRef>, ParserInternalError> {
        let kind = self.kind();
        let symbol_kind = match kind {
            AstKind::DomainName => SymbolKind::DomainName,
            AstKind::PrimitiveType => SymbolKind::PrimitiveType,
            AstKind::ProblemName => SymbolKind::ProblemName,
            AstKind::Constant => SymbolKind::Constant,
            AstKind::Variable => SymbolKind::Variable,
            AstKind::FunctionSymbol => SymbolKind::Function,
            AstKind::Predicate => SymbolKind::Predicate,
            AstKind::ActionSymbol => SymbolKind::Action,
            AstKind::DASymbol => SymbolKind::DASymbol,
            AstKind::MethodSymbol => SymbolKind::Method,
            AstKind::TaskSymbol => SymbolKind::Task,
            AstKind::TaskID => SymbolKind::TaskID,
            _ => return Ok(None),
        };

        let ident = self.try_ident()?;
        Ok(Some(SymbolRef::new(ident, symbol_kind)))
    }

    /// Recursively pretty-prints this node and its subtree using a tree layout.
    fn fmt_with_interner(
        &self,
        f: &mut Formatter<'_>,
        arena: &Arena<Self>,
        interner: &StringInterner,
    ) -> fmt::Result {
        renderer::tree::render(self, f, arena, interner)
    }

    /// Pretty-prints this node and its subtree as planning syntax with indentation.
    fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        arena: &Arena<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        renderer::syntax::render_with_indent(self, f, arena, interner, indent)?;
        Ok(())
    }
}
