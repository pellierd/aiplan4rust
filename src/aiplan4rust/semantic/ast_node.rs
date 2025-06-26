use std::collections::HashMap;
use std::fmt;
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::tree::{TreeArena, NodeId, TreeNode, AbstractNode};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::symbol::{SymbolKind, SymbolRef};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::syntax::elements::{Ident, Requirement};

/// Represents a node in an Abstract Syntax Tree (AST) tree.
///
/// Each `Node` holds information about its kind (syntax type), the source
/// code span it covers, its children nodes (by indices), and optionally
/// its parent node index.
///
/// This structure is designed for tree-based AST storage, where nodes
/// reference each other by indices rather than pointers.
///
/// # Fields
///
/// - `kind`: The type of AST node (e.g., expression, statement).
/// - `children`: A vector of indices pointing to child nodes.
/// - `span`: The source code span this node covers.
/// - `parent`: An optional index of the parent node. `None` if this node is a root.
///
/// # Examples
///
/// ```rust
/// use crate::aiplan4rust::syntax::{AstKind, Span};
/// use crate::aiplan4rust::semantic::tree::Node;
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
pub struct AstArenaNode {
    data: AbstractNode<AstKind, AstContent>,
    span: Span,
}

impl AstArenaNode {
    /// Creates a new `AstArenaNode` with the given kind, content, span, and optional parent.
    ///
    /// The node is initialized without children.
    pub fn new(kind: AstKind, content: AstContent, span: Span, parent: Option<NodeId>) -> Self {
        let data = AbstractNode::new(kind, content, parent);
        AstArenaNode { data, span }
    }

    /// Returns a reference to the source code span of this node.
    pub fn span(&self) -> &Span {
        &self.span
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

    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        match self.content_mut() {
            AstContent::Ident(id) => {
                if let Some(&new_id) = map.get(id) {
                    *id = new_id;
                }
            }
            _ => {}
        }
    }

    fn try_simple_symbol_ref(node: &Self) -> Result<SymbolRef, ParserInternalError> {
        let kind = node.kind();
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
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Node kind {:?} is not a simple symbol.",
                    kind
                )))
            }
        };

        let ident = node.try_ident()?;
        Ok(SymbolRef::new(ident, symbol_kind))
    }

    fn try_signature_symbol_ref(
        node: &Self,
        arena: &TreeArena<Self>,
    ) -> Result<SymbolRef, ParserInternalError> {
        let first_child_id = *node.children().first().ok_or_else(|| {
            ParserInternalError::new(format!(
                "{} must have children, but none found.",
                node.kind()
            ))
        })?;

        let child = arena.try_node(first_child_id)?;
        let child_kind = child.kind();

        match child_kind {
            AstKind::Predicate => {
                let ident = child.try_ident()?;
                Ok(SymbolRef::new(ident, SymbolKind::Predicate))
            }
            AstKind::FunctionSymbol => {
                let ident = child.try_ident()?;
                Ok(SymbolRef::new(ident, SymbolKind::Function))
            }
            AstKind::TaskSymbol => {
                let ident = child.try_ident()?;
                Ok(SymbolRef::new(ident, SymbolKind::Task))
            }
            AstKind::TotalTime => {
                Ok(SymbolRef::new(StringInterner::IDENT_TOTAL_TIME, SymbolKind::Function))
            }
            other => Err(ParserInternalError::new(format!(
                "First child of {:?} must be Predicate, FunctionSymbol or TaskSymbol, found: {:?}",
                node.kind(),
                other
            ))),
        }
    }
}

impl Deref for AstArenaNode {
    type Target = AbstractNode<AstKind, AstContent>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for AstArenaNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl fmt::Display for AstArenaNode {
    /// Formats the `Node` for display purposes.
    ///
    /// Prints a concise summary of the node, including:
    /// - The kind of the node (`kind`)
    /// - The source code span associated with the node (`span`)
    /// - The index of the parent node if it exists (`parent`)
    /// - The list of children node indices (`children`)
    ///
    /// # Example
    ///
    /// ```rust
    /// // Assuming `node` is an instance of `Node`
    /// println!("{}", node);
    /// // Output example:
    /// // Node(kind=PrimitiveType("t1"), span={ start: 0, end: 5 }, parent=None, children=[1, 2])
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let children_str = self.children()
            .iter()
            .map(|idx| idx.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "Node(kind={:?}, content={}, span={:?}, parent={:?}, children=[{}])",
            self.kind(), self.content(), self.span(), self.parent(), children_str
        )
    }
}

impl TreeNode for AstArenaNode {
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

    fn parent(&self) -> Option<NodeId> {
        self.data.parent()
    }

    fn children(&self) -> &[NodeId] {
        self.data.children()
    }

    fn add_child(&mut self, child: NodeId) {
        self.data.add_child(child)
    }

    fn set_parent(&mut self, parent: Option<NodeId>) {
        self.data.set_parent(parent)
    }

    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        self.remap_idents(map)
    }

    fn try_symbol_ref(&self, arena: &TreeArena<Self>) -> Result<SymbolRef, ParserInternalError> {
        let kind = self.kind();

        if matches!(
            kind,
            AstKind::DomainName
                | AstKind::PrimitiveType
                | AstKind::ProblemName
                | AstKind::Constant
                | AstKind::Variable
                | AstKind::FunctionSymbol
                | AstKind::Predicate
                | AstKind::ActionSymbol
                | AstKind::DASymbol
                | AstKind::MethodSymbol
                | AstKind::TaskSymbol
                | AstKind::TaskID
        ) {
            return Self::try_simple_symbol_ref(self);
        }

        if matches!(
            kind,
            AstKind::AtomicFormula | AstKind::FunctionTerm | AstKind::Task
        ) {
            return Self::try_signature_symbol_ref(self, arena);
        }

        Err(ParserInternalError::new(format!(
            "Unexpected symbol kind encountered: {:?}",
            kind
        )))
    }

    fn content_mut(&mut self) -> &mut Self::Content {
        self.data.content_mut()
    }
}
