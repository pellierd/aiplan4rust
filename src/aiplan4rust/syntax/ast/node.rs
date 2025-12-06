//! AST Node Module
//!
//! This module defines the [`AstNode`] structure used to represent elements of the
//! Abstract Syntax Tree (AST) in the parsing and semantic analysis pipeline.
//!
//! # Overview
//! Each `AstNode` holds:
//! - A [`SyntaxBaseNode`] that provides its syntactic kind, content, children, and parent.
//! - A [`Span`] indicating its position in the source code.
//!
//! `AstNode` implements several traits:
//! - [`SyntaxNode`] for AST traversal and rendering.
//! - [`ArenaNode`] to integrate with arena-based memory allocation.
//! - [`Deref`] / [`DerefMut`] for seamless access to its inner node.
//!
//! # Use Cases
//! - Used during parsing to build a tree representation of the program.
//! - Used in semantic analysis and symbol resolution.
//!
//! # Errors
//! Methods like [`AstNode::try_requirement`] and [`AstNode::as_symbol`] may return
//! [`AstError`] or [`SyntaxTreeError`] when semantic constraints are violated.

use std::collections::HashMap;
use std::fmt::{self, Formatter};
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::{Ident, Requirement};
use crate::aiplan4rust::semantic::symbol::{Symbol, SymbolKind};
use crate::aiplan4rust::syntax;
use crate::aiplan4rust::syntax::ast::{renderer, AstContent, AstError, AstKind};
use crate::aiplan4rust::syntax::tree::{SyntaxBaseNode, SyntaxNode, SyntaxTree, NodeId};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::tree::renderers::RenderKind;

/// Represents a node in the Abstract Syntax Tree (AST).
///
/// `AstNode` combines syntactic information (node kind and content)
/// with source metadata (text span) and hierarchical structure
/// (children and parent IDs).
///
/// # Fields
/// - `inner`: A generic syntax base node holding the kind, content, and hierarchy.
/// - `span`: A `Span` representing the start and end positions of the node in the source.
///
/// # Trait Implementations
/// Implements [`SyntaxNode`] and [`ArenaNode`] to allow AST manipulation and traversal.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AstNode {
    inner: SyntaxBaseNode<AstKind, AstContent>,
    span: Span,
}

impl AstNode {
    /// Creates a new `AstNode` instance.
    ///
    /// # Arguments
    /// * `kind` - The syntactic kind of the node (e.g., predicate, variable).
    /// * `content` - The semantic content of the node (e.g., identifier, expression).
    /// * `children` - A list of child node IDs representing the node's subtree.
    /// * `span` - The source code span that this node covers.
    /// * `parent` - Optional parent node ID, if already known.
    ///
    /// # Returns
    /// A newly constructed `AstNode` with the given attributes.
    pub fn new(
        kind: AstKind,
        content: AstContent,
        children: Vec<NodeId>,
        span: Span,
        parent: Option<NodeId>,
    ) -> Self {
        let inner = SyntaxBaseNode::new(kind, content, children, parent);
        AstNode { inner, span }
    }

    /// Returns an immutable reference to the span of this node.
    ///
    /// # Returns
    /// A reference to the [`Span`] representing this node’s source location.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns a mutable reference to the span of this node.
    ///
    /// # Returns
    /// A mutable reference to the [`Span`] representing this node’s source location.
    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    /// Tries to convert the node’s content into a `Requirement`, if possible.
    ///
    /// # Returns
    /// * `Some(Requirement)` if the content can be interpreted as a requirement.
    /// * `None` otherwise.
    pub fn as_requirement(&self) -> Option<Requirement> {
        self.content().as_requirement()
    }

    /// Attempts to extract a `Requirement` from the node’s content.
    ///
    /// # Returns
    /// * `Ok(Requirement)` if extraction is successful.
    /// * `Err(AstError)` if the content is not a valid requirement.
    pub fn try_requirement(&self) -> Result<Requirement, AstError> {
        self.content().try_requirement()
    }
}


impl Deref for AstNode {
    type Target = SyntaxBaseNode<AstKind, AstContent>;

    /// Returns an immutable reference to the inner `SyntaxBaseNode`.
    ///
    /// This enables accessing methods and fields of the inner node directly through `AstNode`.
    ///
    /// # Returns
    /// A reference to the wrapped `SyntaxBaseNode<AstKind, AstContent>`.
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AstNode {
    /// Returns a mutable reference to the inner `SyntaxBaseNode`.
    ///
    /// This enables modifying the fields of the wrapped node directly through `AstNode`.
    ///
    /// # Returns
    /// A mutable reference to the wrapped `SyntaxBaseNode<AstKind, AstContent>`.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl fmt::Display for AstNode {
    /// Formats the AST node using the default renderer.
    ///
    /// This implementation provides a readable string representation of the node, including
    /// its kind, content, and optionally its span or children.
    ///
    /// # Arguments
    /// * `f` – The formatter used to write the output.
    ///
    /// # Returns
    /// A `fmt::Result` indicating success or failure of the formatting operation.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderer::default::render(self, f)
    }
}

impl ArenaNode for AstNode {
    /// Returns the ID of this node's parent, if any.
    ///
    /// # Returns
    /// - `Some(NodeId)` if the node has a parent in the syntax tree.
    /// - `None` if the node is a root or has not been assigned a parent.
    fn parent(&self) -> Option<NodeId> {
        self.inner.parent()
    }

    /// Sets the parent of this node.
    ///
    /// # Arguments
    /// - `parent`: An optional `NodeId` referencing the parent node in the arena.
    ///   Use `None` to indicate that this node has no parent.
    fn set_parent(&mut self, parent: Option<NodeId>) {
        self.inner.set_parent(parent);
    }

    /// Returns a slice of child node IDs.
    ///
    /// # Returns
    /// A slice (`&[NodeId]`) of all direct children of this node.
    fn children(&self) -> &[NodeId] {
        self.inner.children()
    }

    /// Replaces this node's children with a new list of children.
    ///
    /// # Arguments
    /// - `children`: A `Vec<NodeId>` containing the IDs of the new children to assign to this node.
    fn set_children(&mut self, children: Vec<NodeId>) {
        self.inner.set_children(children);
    }

    /// Adds a single child node to this node.
    ///
    /// # Arguments
    /// - `child`: The `NodeId` of the child node to add.
    ///
    /// This appends the child to the end of the current children list.
    fn add_child(&mut self, child: NodeId) {
        self.inner.add_child(child);
    }
}

impl SyntaxNode for AstNode {
    type Kind = AstKind;
    type Content = AstContent;

    /// Returns the kind (`AstKind`) of this AST node.
    ///
    /// # Returns
    /// A copy of the node's kind representing its syntactic category.
    fn kind(&self) -> Self::Kind {
        self.inner.kind()
    }

    /// Sets the kind (`AstKind`) of this AST node.
    ///
    /// # Arguments
    /// - `kind`: The new kind to assign to the node.
    fn set_kind(&mut self, kind: Self::Kind) {
        self.inner.set_kind(kind);
    }

    /// Returns the rendering kind of this AST node.
    ///
    /// The `render_kind` provides a high-level categorization of the node
    /// that is used by renderers to determine how to display it. This
    /// abstracts over the specific underlying AST and maps it to a `RenderKind` variant.
    ///
    /// # Returns
    /// A `RenderKind` value representing the node’s appearance in rendered
    /// output. This is typically used by syntax tree renderers or formatters
    /// to decide keywords, indentation, or visual representation.
    fn render_kind(&self) -> RenderKind {
        RenderKind::from_ast_kind(self.kind())
    }

    /// Returns a shared reference to the node's semantic content.
    ///
    /// # Returns
    /// A reference to the `AstContent` contained in this node.
    fn content(&self) -> &Self::Content {
        self.inner.content()
    }

    /// Replaces the content of this AST node.
    ///
    /// # Arguments
    /// * `content` - The new content to assign.
    fn set_content(&mut self, content: AstContent) {
        self.inner.set_content(content);
    }

    /// Returns a mutable reference to the node's semantic content.
    ///
    /// # Returns
    /// A mutable reference to the `AstContent` allowing in-place modification.
    fn content_mut(&mut self) -> &mut Self::Content {
        self.inner.content_mut()
    }

    /// Converts this node into an optional `SymbolRef`, if applicable.
    ///
    /// This is only possible for nodes that represent symbol-bearing constructs
    /// such as identifiers (e.g. predicates, types, tasks).
    ///
    /// # Returns
    /// - `Ok(Some(Symbol))` if the node's kind maps to a symbol kind and has an associated identifier.
    /// - `Ok(None)` if the node kind is not symbol-bearing.
    /// - `Err(SyntaxTreeError)` if the node is malformed or missing an expected identifier.
    fn as_symbol(&self) -> Result<Option<Symbol>, SyntaxTreeError> {
        let symbol_kind = match self.kind() {
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
        Ok(Some(Symbol::new(ident, symbol_kind)))
    }

    /// Replaces the identifier of this node with a remapped value, if applicable.
    ///
    /// This only affects nodes whose content is an identifier (`AstContent::Ident`).
    ///
    /// # Arguments
    /// - `map`: A map from old `Ident`s to new ones. If this node's ident is in the map,
    ///   it will be replaced.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        if let AstContent::Ident(id) = self.content_mut() {
            if let Some(&new_id) = map.get(id) {
                *id = new_id;
            }
        }
    }

    /// Creates a shallow clone of the node.
    ///
    /// This clone copies the node's kind, content, and span, but **does not include**
    /// its parent or children. The resulting node has an empty children list and no parent.
    ///
    /// Typically used when reconstructing a subtree with [`SyntaxTree::clone_subtree`],
    /// where each node is cloned individually before linking to new parent nodes.
    ///
    /// # Returns
    /// A new `AstNode` with the same kind, content, and span, but without parent or children.
    ///
    /// # Example
    /// ```ignore
    /// let node: AstNode = ...;
    /// let shallow = node.clone_shallow();
    /// assert_eq!(shallow.kind(), node.kind());
    /// assert_eq!(shallow.content(), node.content());
    /// assert!(shallow.children().is_empty());
    /// assert!(shallow.parent().is_none());
    /// ```
    fn clone_shallow(&self) -> Self {
        AstNode {
            inner: SyntaxBaseNode::new(
                self.inner.kind(),
                self.inner.content().clone(),
                Vec::new(),
                None,
            ),
            span: self.span.clone(),
        }
    }

    /// Returns `true` if the node represents an **atomic formula**.
    ///
    /// In the AST, this typically corresponds to either:
    /// - `AtomicFormula`: a basic predicate or proposition.
    /// - `FComp`: a fluent comparison.
    ///
    /// # Example
    /// ```
    /// assert!(node.is_atomic_formula());
    /// ```
    fn is_atomic_formula(&self) -> bool {
        matches!(self.kind(), AstKind::AtomicFormula | AstKind::FComp)
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
        matches!(self.kind(), AstKind::AtStart | AstKind::AtEnd | AstKind::Overall)
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
        matches!(self.kind(), AstKind::And | AstKind::Or | AstKind::Not | AstKind::Imply)
    }

    /// Returns `true` if this node represents a logical negation (`Not`).
    ///
    /// This enables generic algorithms (such as literal detection)
    /// to operate independently of the specific `Kind` enum used.
    fn is_not(&self) -> bool {
        matches!(self.kind(), AstKind::Not)
    }

    /// Formats the AST node for display, using a syntax tree and string interner.
    ///
    /// # Arguments
    /// - `f`: The formatter to write to.
    /// - `syntax_tree`: The full syntax tree containing this node and its children.
    /// - `interner`: The string interner used to resolve identifier strings.
    ///
    /// # Returns
    /// A `fmt::Result` indicating whether formatting succeeded.
    fn fmt_with_interner(
        &self,
        f: &mut Formatter<'_>,
        syntax_tree: &SyntaxTree<Self>,
        interner: &StringInterner,
    ) -> fmt::Result {
        renderer::tree::render(self, f, syntax_tree, interner)
    }

    /// Formats the AST node with indentation for nested structure representation.
    ///
    /// # Arguments
    /// - `f`: The formatter to write to.
    /// - `syntax_tree`: The syntax tree that contains the node and its children.
    /// - `interner`: The interner used to resolve identifier names.
    /// - `indent`: The current indentation level (used for pretty-printing).
    ///
    /// # Returns
    /// A `fmt::Result` indicating whether formatting succeeded.
    fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        syntax_tree: &SyntaxTree<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        syntax::tree::renderers::syntax_rendering(self, f, syntax_tree, interner)
//        renderer::syntax::render_with_indent(self, f, syntax_tree, interner, indent)
    }
}
