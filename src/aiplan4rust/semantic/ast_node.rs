use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::tree::{TreeArena, NodeId, TreeNode, AbstractNode};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::semantic::symbol::{SymbolKind, SymbolRef};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::syntax::ast::content::Content;
use crate::aiplan4rust::syntax::elements::Requirement;
use crate::aiplan4rust::lang::Ident;

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
/// - `kind`: The type of AST node (e.g., expr, statement).
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
        let children = self.children()
            .iter()
            .map(|idx| idx.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let parent = self.parent()
            .map_or("none".to_string(), |idx| idx.to_string());

        let span = self.span();
        let span_str = format!(
            "[l{}:c{}-l{}:c{}]",
            span.begin_line(),
            span.begin_column(),
            span.end_line(),
            span.end_column()
        );

        write!(
            f,
            "[kind={}, content={}, span={} parent={}, children=[{}]]",
            self.kind(),
            self.content(),
            span_str,
            parent,
            children,
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

    fn add_child(&mut self, child: NodeId) {
        self.data.add_child(child)
    }

    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        self.remap_idents(map)
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

    /// Recursively pretty-prints this node and its children as a tree.
    fn fmt_with(
        &self,
        f: &mut Formatter<'_>,
        arena: &TreeArena<Self>,
        interner: &StringInterner,
    ) -> fmt::Result {
        fn fmt_node(
            node: &AstArenaNode,
            f: &mut Formatter<'_>,
            arena: &TreeArena<AstArenaNode>,
            interner: &StringInterner,
            prefix: &str,
            last: bool,
        ) -> fmt::Result {
            let branch = if last { "└─" } else { "├─" };

            let content_str = match node.content() {
                Content::None => String::new(),
                Content::Ident(id) => format!(" [{}]", id.to_string_with_interner(interner)),
                other => format!(" [{}]", other),
            };

            let (line, column) = node.span().start_position();
            let span_str = format!(" (l{}:c{})", line, column);

            // On différencie si c'est la racine ultime ou pas
            let children = node.children();
            let len = children.len();

            // Écrire la ligne courante
            write!(
                f,
                "{}{}{}{}{}",
                prefix,
                branch,
                node.kind(),
                content_str,
                span_str
            )?;

            // Si ce nœud a des enfants, on passe à la ligne
            if !children.is_empty() {
                writeln!(f)?;
            }

            let new_prefix = if last {
                format!("{}   ", prefix)
            } else {
                format!("{}│  ", prefix)
            };

            // Parcourir les enfants
            for (i, child_idx) in children.iter().enumerate() {
                let child = arena
                    .get_node(*child_idx)
                    .expect("Child not found in arena");
                fmt_node(child, f, arena, interner, &new_prefix, i == len - 1)?;

                // Si ce n'est pas le dernier enfant, retour à la ligne après chaque sous-arbre
                if i < len - 1 {
                    writeln!(f)?;
                }
            }

            Ok(())
        }

        fmt_node(self, f, arena, interner, "", true)
    }






    fn fmt_syntax(&self, f: &mut Formatter<'_>, arena: &TreeArena<Self>, interner: &StringInterner) -> fmt::Result
    where
        Self: Sized
    {
        self.fmt_with(f, arena, interner)
    }
}
