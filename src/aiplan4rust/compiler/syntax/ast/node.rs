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
//! - [`Node`] for AST traversal and rendering.
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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Formatter};
use std::ops::{Deref, DerefMut};

use crate::aiplan4rust::compiler::semantic::symbol::{Symbol, SymbolKind};
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::compiler::syntax::ast::tree::{Node, NodeId, SyntaxBaseNode, Tree};
use crate::aiplan4rust::compiler::syntax::ast::{renderer, AstContent, AstError, AstKind};
use crate::aiplan4rust::compiler::syntax::Span;
use crate::aiplan4rust::support::interner::{InternerError, SymbolInterner};
use crate::aiplan4rust::support::lang::{RemapSymbol, Requirement, SymbolId};

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
/// Implements [`Node`] and [`ArenaNode`] to allow AST manipulation and traversal.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AstNode {
    inner: SyntaxBaseNode<AstKind, AstContent>,
    span: Span,
}

impl AstNode {
    /// Reserved AST node index for the PDDL 'object' root type.
    /// Matches [`SymbolInterner::OBJECT_SYMBOL_ID`].
    pub const NODE_ID_OBJECT: NodeId = NodeId::new(0);

    /// Reserved AST node index for the PDDL 'number' primitive type.
    /// Matches [`SymbolInterner::NUMBER_SYMBOL_ID`].
    pub const NODE_ID_NUMBER: NodeId = NodeId::new(1);

    /// Reserved AST node index for the implicit '?duration' variable.
    /// This node is automatically available within the scope of durative actions.
    /// Matches [`SymbolInterner::DURATION_VARIABLE_SYMBOL_ID`].
    pub const NODE_ID_DURATION: NodeId = NodeId::new(2);

    /// Reserved AST node index for the 'total-time' system function.
    /// Typically used in problem metrics for temporal planning.
    /// Matches [`SymbolInterner::TOTAL_TIME_SYMBOL_ID`].
    pub const NODE_ID_TOTAL_TIME: NodeId = NodeId::new(3);

    /// Reserved AST node index for the 'total-cost' system function.
    /// Used when the `:action-costs` requirement is active.
    /// Matches [`SymbolInterner::TOTAL_COST_SYMBOL_ID`].
    pub const NODE_ID_TOTAL_COST: NodeId = NodeId::new(4);

    /// Reserved AST node index for the '#t' continuous time variable.
    /// Used in PDDL+ for continuous effects and processes.
    /// Matches [`SymbolInterner::CONTINUOUS_VARIABLE_SYMBOL_ID`].
    pub const NODE_ID_CONTINUOUS_TIME: NodeId = NodeId::new(5);

    /// The total number of built-in nodes reserved at the start of the AST arena.
    const BUILTIN_NODES_COUNT: usize = 6;

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

    /// Returns `true` if the node is a language-intrinsic (built-in) node.
    ///
    /// Built-in nodes (such as 'object' or 'number') are allocated at startup
    /// and exist as roots in the AST arena without any parent.
    #[inline]
    pub fn is_builtin(id: NodeId) -> bool {
        // We use the constant as an exclusive upper bound since built-ins
        // are allocated first in the arena (indices 0 to N-1).
        id.as_usize() < Self::BUILTIN_NODES_COUNT
    }

    /// Returns the source code [`Span`] of this node.
    ///
    /// Since [`Span`] implements [`Copy`], this method returns the span by value.
    ///
    /// # Returns
    /// The [`Span`] representing this node’s source location.
    pub fn span(&self) -> Span {
        self.span
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

    pub fn as_ident(&self) -> Option<SymbolId> {
        self.content().as_ident()
    }

    pub fn try_ident(&self) -> Result<SymbolId, AstError> {
        self.content().try_ident()
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
    fn as_symbol(&self) -> Result<Option<Symbol>, AstError> {
        let symbol_kind = match self.kind() {
            AstKind::DomainName => SymbolKind::DomainName,
            AstKind::PrimitiveType => SymbolKind::PrimitiveType,
            AstKind::ProblemName => SymbolKind::ProblemName,
            AstKind::Object => SymbolKind::Constant,
            AstKind::Variable => SymbolKind::Variable,
            AstKind::FunctionSymbol => SymbolKind::Function,
            AstKind::PredicateSymbol => SymbolKind::Predicate,
            AstKind::ActionSymbol => SymbolKind::Action,
            AstKind::DASymbol => SymbolKind::DASymbol,
            AstKind::MethodSymbol => SymbolKind::Method,
            AstKind::TaskSymbol => SymbolKind::Task,
            AstKind::TaskLabel => SymbolKind::TaskID,
            _ => return Ok(None),
        };

        let ident = self.try_ident()?;
        Ok(Some(Symbol::new(ident, symbol_kind)))
    }

    /// Attempts to extract a symbol  from the syntax’s content.
    ///
    /// # Returns
    ///
    /// A `Result<Symbol, SyntaxTreeError>` containing the symbol if successful,
    /// or an error if extraction failed or no symbol is present.
    pub(crate) fn try_symbol(&self) -> Result<Symbol, AstError> {
        self.as_symbol()?.ok_or_else(|| AstError::not_a_symbol_id())
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
    /// Formats the AST node using the debug renderer.
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

impl Node for AstNode {
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

    /// Returns a shared reference to the node's semantic content.
    ///
    /// # Returns
    /// A reference to the `AstContent` contained in this node.
    fn content(&self) -> &Self::Content {
        self.inner.content()
    }

    /// Returns a mutable reference to the node's semantic content.
    ///
    /// # Returns
    /// A mutable reference to the `AstContent` allowing in-place modification.
    fn content_mut(&mut self) -> &mut Self::Content {
        self.inner.content_mut()
    }

    /// Replaces the content of this AST node.
    ///
    /// # Arguments
    /// * `content` - The new content to assign.
    fn set_content(&mut self, content: AstContent) {
        self.inner.set_content(content);
    }

    /// Creates a shallow clone of the node.
    ///
    /// This clone copies the node's kind, content, and span, but **does not include**
    /// its parent or children. The resulting node has an empty children list and no parent.
    ///
    /// Typically used when reconstructing a subtree with [`Tree::clone_subtree`],
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
        matches!(self.kind(), AstKind::AtomicFormula | AstKind::Comparison)
    }

    /// Returns `true` if the node is a **temporal specifier**.
    ///
    /// Temporal specifiers are nodes like:
    /// - `AtStart`
    /// - `AtEnd`
    /// - `Overall`
    ///
    /// They indicate when the literal should hold in PDDL temporal logic.
    ///
    /// # Example
    /// ```
    /// assert!(node.is_time_specifier());
    /// ```
    fn is_time_specifier(&self) -> bool {
        matches!(
            self.kind(),
            AstKind::AtStart | AstKind::AtEnd | AstKind::Overall
        )
    }

    /// Returns `true` if the node represents a **logical operator**.
    ///
    /// Logical operators include:
    /// - `And`
    /// - `Or`
    /// - `Not`
    /// - `Imply`
    ///
    /// Useful for expression traversal, logic, and propagation of temporal specifiers.
    ///
    /// # Example
    /// ```
    /// assert!(node.is_logic());
    /// ```
    fn is_logic(&self) -> bool {
        matches!(
            self.kind(),
            AstKind::And | AstKind::Or | AstKind::Not | AstKind::Imply
        )
    }

    /// Returns `true` if this node represents a logical negation (`Not`).
    ///
    /// This enables generic algorithms (such as literal detection)
    /// to operate independently of the specific `Kind` enum used.
    fn is_not(&self) -> bool {
        matches!(self.kind(), AstKind::Not)
    }

    /// Returns `true` if this node represents a logical variable.
    ///
    /// This is used during the flattening and grounding process to distinguish
    /// between lifted parameters (variables) and ground objects (constants)
    /// within an atomic formula's arguments.
    ///
    /// # Returns
    /// * `true` if the underlying node kind is a variable.
    /// * `false` otherwise.
    fn is_variable(&self) -> bool {
        matches!(self.kind(), AstKind::Variable)
    }
}

impl RemapSymbol for AstNode {
    /// Remaps identifiers in this syntax node's content using the provided map.
    ///
    /// This debug implementation works for any typing implementing [`Node`],
    /// delegating the remapping to `content_mut()`.
    ///
    /// # Parameters
    /// - `map`: A `HashMap` mapping old [`SymbolId`]s to new ones.
    ///
    /// # Errors
    /// Returns a [`InternerError`] if remapping fails.
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        self.content_mut().remap_symbol(map)?;
        Ok(())
    }
}

impl AstNode {
    // --- Rendu de l'Arbre (Visualisation) ---
    pub fn fmt_with_interner(
        &self,
        f: &mut Formatter<'_>,
        syntax_tree: &Tree<AstNode>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        renderer::tree::render(self, f, syntax_tree, interner)
    }

    pub fn to_string_with_interner(
        &self,
        tree: &Tree<AstNode>,
        interner: &SymbolInterner,
    ) -> String {
        struct Wrapper<'a>(&'a AstNode, &'a Tree<AstNode>, &'a SymbolInterner);
        impl fmt::Display for Wrapper<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.0.fmt_with_interner(f, self.1, self.2)
            }
        }
        format!("{}", Wrapper(self, tree, interner))
    }

    // --- Rendu Syntaxique (PDDL) ---

    pub fn fmt_syntax(
        &self,
        f: &mut Formatter<'_>,
        syntax_tree: &Tree<AstNode>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        // Elle délègue simplement à la version avec indent 0
        self.fmt_syntax_with_indent(f, syntax_tree, interner, 0)
    }

    pub fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        syntax_tree: &Tree<AstNode>,
        interner: &SymbolInterner,
        _indent: usize,
    ) -> fmt::Result {
        // Appelle ton renderer PDDL spécialisé
        renderer::syntax::render(self, f, syntax_tree, interner)
    }

    pub fn to_syntax_string(&self, tree: &Tree<AstNode>, interner: &SymbolInterner) -> String {
        struct Wrapper<'a>(&'a AstNode, &'a Tree<AstNode>, &'a SymbolInterner);
        impl fmt::Display for Wrapper<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.0.fmt_syntax_with_indent(f, self.1, self.2, 0)
            }
        }
        format!("{}", Wrapper(self, tree, interner))
    }
}
