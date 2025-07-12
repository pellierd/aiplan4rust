use crate::aiplan4rust::arena::{Arena, ArenaNode, BaseNode, NodeId};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::semantic::symbol::{SymbolKind, SymbolRef};
use crate::aiplan4rust::syntax::ast::{renderer, AstContent, AstKind};
use crate::aiplan4rust::syntax::Span;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};

/// Represents a node in an Abstract Syntax Tree (AST) arena.
///
/// Each `Node` holds information about its kind (syntax type), the source
/// code span it covers, its children nodes (by indices), and optionally
/// its parent node index.
///
/// This structure is designed for arena-based AST storage, where nodes
/// reference each other by indices rather than pointers.
///
/// # Fields
///
/// - `kind`: The type of AST node (e.g., expr, statement).
/// - `children`: A vector of indices pointing to child nodes.
/// - `span`: The source code span this node covers.
/// - `parent`: An optional index of the parent node. `None` if this node is a root.
///
/// # Examples
///
/// ```rust
/// use crate::aiplan4rust::syntax::{AstKind, Span};
/// use crate::aiplan4rust::semantic::arena::Node;
///
/// let kind = AstKind::Expr; // example variant
/// let span = Span::default();
/// let mut node = Node::new(kind.clone(), span.clone(), None);
///
/// assert!(node.is_root());
/// assert_eq!(node.kind(), &kind);
/// assert_eq!(node.span(), &span);
/// assert_eq!(node.children().len(), 0);
///
/// node.set_kind(AstKind::Stmt);
/// node.set_span(Span::new(10, 20));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AstNode {
    data: BaseNode<AstKind, AstContent>,
    span: Span,
}

impl AstNode {
    /// Creates a new `AstArenaNode` with the given kind, content, span, and optional parent.
    ///
    /// The node is initialized without children.
    pub fn new(
        kind: AstKind,
        content: AstContent,
        children: Vec<NodeId>,
        span: Span,
        parent: Option<NodeId>,
    ) -> Self {
        let data = BaseNode::new(kind, content, children, parent);
        AstNode { data, span }
    }

    /// Returns a reference to the source code span of this node.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns a reference to the source code span of this node.
    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    /// Returns the requirement flag if this content is a `Requirement`.
    ///
    /// # Returns
    ///
    /// - `Some(Requirement)` if the content is a requirement.
    /// - `None` otherwise.
    pub fn as_requirement(&self) -> Option<Requirement> {
        self.content().as_requirement()
    }

    /// Returns the requirement flag if this content is a `Requirement`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not a `Requirement`.
    pub fn try_requirement(&self) -> Result<Requirement, ParserInternalError> {
        self.content().try_requirement()
    }
}

impl Deref for AstNode {
    type Target = BaseNode<AstKind, AstContent>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for AstNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl fmt::Display for AstNode {
    /// Affiche un résumé complet du nœud, utile pour le debug ou logs.
    ///
    /// Affiche :
    /// - le type/kind du nœud,
    /// - le contenu (`content`),
    /// - la position source (`span`),
    /// - le parent s'il existe,
    /// - la liste des enfants (indices).
    ///
    /// # Exemple
    /// ```rust
    /// println!("{}", node);
    /// // Node[kind=PrimitiveType("t1"), content=..., span=..., parent=none, children=[1, 2]]
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        renderer::default::render(self, f)
    }
}

impl ArenaNode for AstNode {
    type Kind = AstKind;
    type Content = AstContent;

    fn kind(&self) -> Self::Kind {
        self.data.kind()
    }

    fn set_kind(&mut self, kind: Self::Kind) {
        self.data.set_kind(kind);
    }

    fn content(&self) -> &Self::Content {
        &self.data.content()
    }

    fn content_mut(&mut self) -> &mut Self::Content {
        self.data.content_mut()
    }

    fn parent(&self) -> Option<NodeId> {
        self.data.parent()
    }

    fn set_parent(&mut self, parent: Option<NodeId>) {
        self.data.set_parent(parent)
    }

    fn children(&self) -> &[NodeId] {
        self.data.children()
    }

    fn set_children(&mut self, children: Vec<NodeId>) {
        self.data.set_children(children)
    }

    fn add_child(&mut self, child: NodeId) {
        self.data.add_child(child)
    }

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

    /// Recursively pretty-prints this node and its children as a arena.
    fn fmt_with(
        &self,
        f: &mut Formatter<'_>,
        arena: &Arena<Self>,
        interner: &StringInterner,
    ) -> fmt::Result {
        renderer::tree::render(self, f, arena, interner)
    }

    fn fmt_planning_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        arena: &Arena<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {

        renderer::syntax::render(self, f, arena, interner, indent)?;
        Ok(())
    }
}
