use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::semantic::symbol::{SymbolKind, SymbolRef};
use crate::aiplan4rust::syntax::ast::content::Content;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::syntax::{PlanningSyntaxDisplay, Span};
use crate::aiplan4rust::tree::{AbstractNode, NodeId, TreeArena, TreeNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};
use crate::aiplan4rust::syntax::lexer::Token;

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

    fn fmt_typed_list(
        &self,
        f: &mut Formatter<'_>,
        arena: &TreeArena<Self>,
        interner: &StringInterner,
        multiline: bool,
    ) -> fmt::Result {
        for (i, child_id) in self.children().iter().enumerate() {
            if i > 0 && !multiline {
                write!(f, " ")?;
            }
            if let Some(child_node) = arena.get_node(*child_id) {
                if multiline {
                    write!(f, "    ")?; // indentation
                }
                child_node.fmt_planning(f, arena, interner)?;
                if multiline {
                    writeln!(f)?;
                }
            }
        }
        Ok(())
    }

    fn fmt_task(
        &self,
        f: &mut Formatter<'_>,
        arena: &TreeArena<Self>,
        interner: &StringInterner,
        with_prefix: bool,
    ) -> fmt::Result {
        if with_prefix {
            write!(f, "  :task ")?;
        }

        let children = self.children();
        if children.is_empty() {
            write!(f, "()")?;
        } else {
            write!(f, "(")?;
            for (i, child_id) in children.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                if let Some(child_node) = arena.get_node(*child_id) {
                    child_node.fmt_planning(f, arena, interner)?;
                } else {
                    write!(f, "<invalid>")?;
                }
            }
            write!(f, ")")?;
        }

        if with_prefix {
            writeln!(f)
        } else {
            Ok(())
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

    fn fmt_planning(
        &self,
        f: &mut Formatter<'_>,
        arena: &TreeArena<Self>,
        interner: &StringInterner,
    ) -> fmt::Result {
        match self.kind() {
            AstKind::Domain => {
                write!(f, "(define (domain ")?;
                if let Some(name_node) = self.children().get(0).and_then(|id| arena.get_node(*id)) {
                    name_node.fmt_planning(f, arena, interner)?;
                }
                write!(f, ")")?;
                for child in self.children().iter().skip(1) {
                    if let Some(child_node) = arena.get_node(*child) {
                        writeln!(f)?;
                        child_node.fmt_planning(f, arena, interner)?;
                    }
                }
                write!(f, "\n)")
            }

            AstKind::Problem => {
                write!(f, "(define (problem ")?;
                if let Some(name_node) = self.children().get(0).and_then(|id| arena.get_node(*id)) {
                    name_node.fmt_planning(f, arena, interner)?;
                }
                writeln!(f, ")")?;
                for child in self.children().iter().skip(1) {
                    if let Some(child_node) = arena.get_node(*child) {
                        writeln!(f)?;
                        child_node.fmt_planning(f, arena, interner)?;
                    }
                }
                write!(f, ")")
            }

            AstKind::RequireDef => {
                write!(f, "  (:requirements")?;
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                    }
                }
                write!(f, ")")
            }

            AstKind::TypesDef => {
                writeln!(f, "  (:types")?;
                if let Some(child_id) = self.children().first() {
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_typed_list(f, arena, interner, true)?; // ligne par ligne
                    }
                }
                write!(f, "  )")
            }

            AstKind::TypedList => {
                self.fmt_typed_list(f, arena, interner, false)
            }

            AstKind::TypedItemElements => {
                for (i, child_id) in self.children().iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                    }
                }
                Ok(())
            }

            AstKind::TypedItem => {
                let children = self.children();
                let n = children.len();

                if n == 0 {
                    // Rien à afficher
                    return Ok(());
                }

                // Tous sauf le dernier sont des typedElements
                for (i, child_id) in children.iter().take(n - 1).enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;

                    } else {
                        write!(f, "<invalid_node>")?;
                    }
                }

                // S'il y a plus d'un élément, on affiche " - " avant le type
                if n > 1 {
                    write!(f, " - ")?;
                    if let Some(type_node) = arena.get_node(children[n - 1]) {
                        type_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid_node>")?;
                    }
                }

                Ok(())
            }
            AstKind::Type => {
                match self.children().len() {
                    0 => write!(f, ""),
                    1 => {
                        let ty_node_id = self.children()[0];
                        if let Some(ty_node) = arena.get_node(ty_node_id) {
                            ty_node.content().fmt_planning(f, interner)
                        } else {
                            write!(f, "<invalid_node>")
                        }
                    }

                    _ => {
                        write!(f, "(either")?;
                        for child_id in self.children() {
                            write!(f, " ")?;
                            if let Some(ty_node) = arena.get_node(*child_id) {
                                ty_node.content().fmt_planning(f, interner)?;
                            } else {
                                write!(f, "<invalid_node>")?;
                            }
                        }
                        write!(f, ")")
                    }
                }
            }

            AstKind::ConstantsDef => {
                writeln!(f, "  (:constants")?;
                if let Some(child_id) = self.children().first() {
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_typed_list(f, arena, interner, true)?; // ligne par ligne
                    }
                }
                write!(f, "  )")
            }

            AstKind::PredicatesDef => {
                writeln!(f, "  (:predicates")?;
                for child_id in self.children() {
                    if let Some(child_node) = arena.get_node(*child_id) {
                        write!(f, "  ")?;
                        child_node.fmt_planning(f, arena, interner)?;
                        writeln!(f)?;
                    }
                }
                write!(f, "  )")
            }

            AstKind::FunctionsDef => {
                write!(f, "  (:functions")?;
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                    }
                }
                write!(f, ")")
            }

            AstKind::AtomicFormulaSkeleton
            | AstKind::AtomicFunctionSkeleton => {
                write!(f, "  (")?;
                let mut first = true;
                for child_id in self.children() {
                    if !first {
                        write!(f, " ")?;
                    }
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid_node>")?;
                    }
                    first = false;
                }
                write!(f, ")")
            }

            AstKind::TaskDef => {
                write!(f, "  (:task ")?;

                let mut children_iter = self.children().iter();

                // 1. Le nom
                if let Some(name_id) = children_iter.next() {
                    if let Some(name_node) = arena.get_node(*name_id) {
                        name_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid_name>")?;
                    }
                } else {
                    write!(f, "<missing_name>")?;
                }

                // 2. Les paramètres
                if let Some(params_id) = children_iter.next() {
                    if let Some(params_node) = arena.get_node(*params_id) {
                        write!(f, " :parameters (")?;
                        let mut first = true;
                        for child_id in params_node.children() {
                            if !first {
                                write!(f, " ")?;
                            }
                            if let Some(child_node) = arena.get_node(*child_id) {
                                child_node.fmt_planning(f, arena, interner)?;
                            } else {
                                write!(f, "<invalid_param>")?;
                            }
                            first = false;
                        }
                        write!(f, ")")?;
                    }
                }

                write!(f, ")")
            }

            AstKind::MethodDef => {
                write!(f, "  (:method ")?;

                let mut children_iter = self.children().iter();

                // 1. Nom
                if let Some(name_id) = children_iter.next() {
                    if let Some(name_node) = arena.get_node(*name_id) {
                        name_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid_name>")?;
                    }
                } else {
                    write!(f, "<missing_name>")?;
                }

                // 2. Paramètres
                if let Some(params_id) = children_iter.next() {
                    if let Some(params_node) = arena.get_node(*params_id) {
                        write!(f, "\n    :parameters (")?;
                        let mut first = true;
                        for child_id in params_node.children() {
                            if !first {
                                write!(f, " ")?;
                            }
                            if let Some(child_node) = arena.get_node(*child_id) {
                                child_node.fmt_planning(f, arena, interner)?;
                            } else {
                                write!(f, "<invalid_param>")?;
                            }
                            first = false;
                        }
                        write!(f, ")")?;
                    }
                }

                // 3. Les autres enfants éventuels (par exemple precondition, task network)
                for rest_child_id in children_iter {
                    write!(f, "\n  ")?; // indentation
                    if let Some(rest_child_node) = arena.get_node(*rest_child_id) {
                        rest_child_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid_child>")?;
                    }
                }

                write!(f, "  )")
            }

            AstKind::Task => {
                self.fmt_task(f, arena, interner, true)
            }

            AstKind::PreconditionDef
            | AstKind::EffectDef
            | AstKind::MethodPreconditionDef => {
                write!(f, "    ")?;
                self.kind().fmt_planning(f, interner)?;
                write!(f, "\n      ")?;
                if let Some(first_child_id) = self.children().first() {
                    if let Some(first_child_node) = arena.get_node(*first_child_id) {
                        first_child_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }
                } else {
                    write!(f, "()")?; // si pas de fils, afficher vide
                }
                writeln!(f)
            }
            AstKind::AtomicFormula
            | AstKind::FunctionTerm => {
                write!(f, "(")?;
                let mut first = true;
                for child_id in self.children() {
                    if !first {
                        write!(f, " ")?;
                    }
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }
                    first = false;
                }
                write!(f, ")")
            }

            AstKind::And
            | AstKind::Or
            | AstKind::Not
            | AstKind::Imply
            | AstKind::AtStart
            | AstKind::AtEnd
            | AstKind::Overall
            | AstKind::Assign
            | AstKind::FComp => {
                write!(f, "(")?;
                self.kind().fmt_planning(f, interner)?;
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }
                }
                write!(f, ")")
            }

            AstKind::TaskNetworkDef => {
                for child_id in self.children() {
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                        writeln!(f)?;
                    } else {
                        writeln!(f, "<invalid>")?;
                    }
                }
                Ok(())
            }

            AstKind::OrderedSubtaskDef
            | AstKind::PartiallyOrderedSubtaskDef => {
                // Indique le type de subtask
                write!(f, "    ")?;
                self.kind().fmt_planning(f, interner)?;
                writeln!(f)?;

                let mut children = self.children().iter();

                // Le premier enfant est le AND (ou autre opérateur logique)
                if let Some(and_id) = children.next() {
                    if let Some(and_node) = arena.get_node(*and_id) {
                        let and_children = and_node.children();

                        // Commence la clause (and
                        write!(f, "      (")?;
                        and_node.kind().fmt_planning(f, interner)?; // affiche "and"

                        if and_children.is_empty() {
                            // Cas vide
                            write!(f, " )")?;
                        } else {
                            writeln!(f)?;
                            for child_id in and_children {
                                if let Some(task_node) = arena.get_node(*child_id) {
                                    match task_node.kind() {
                                        AstKind::Task => {
                                            // Affiche un task sans préfixe
                                            write!(f, "       ")?;
                                            task_node.fmt_task(f, arena, interner, false)?;
                                            writeln!(f)?;
                                        }
                                        AstKind::TaggedTask => {
                                            // Affiche un tagged task avec fmt_planning
                                            write!(f, "       ")?;
                                            task_node.fmt_planning(f, arena, interner)?;
                                            writeln!(f)?;
                                        }
                                        other => {
                                            // Si c'est autre chose, on l'indique
                                            writeln!(f, "       <unexpected {:?}>", other)?;
                                        }
                                    }
                                } else {
                                    writeln!(f, "       <invalid>")?;
                                }
                            }
                            write!(f, "      )")?;
                        }
                    } else {
                        writeln!(f, "<invalid-and-node>")?;
                    }
                } else {
                    writeln!(f, "<no-children>")?;
                }

                Ok(())
            }

            AstKind::TaggedTask => {
                let children = self.children();
                if children.len() != 2 {
                    write!(f, "<invalid-tagged-task>")?;
                    return Ok(());
                }

                write!(f, "  (")?;
                // Affiche le TaskID
                if let Some(task_id_node) = arena.get_node(children[0]) {
                    task_id_node.fmt_planning(f, arena, interner)?;
                } else {
                    write!(f, "<invalid-task-id>")?;
                }

                write!(f, " ")?;

                // Affiche le task (sans ":task")
                if let Some(task_node) = arena.get_node(children[1]) {
                    task_node.fmt_task(f, arena, interner, false)?;
                } else {
                    write!(f, "<invalid-task>")?;
                }

                write!(f, ")")
            }

            AstKind::TaskOrderingConstraintDef => {
                write!(f, "    ")?;
                self.kind().fmt_planning(f, interner)?;
                writeln!(f)?;

                let mut children = self.children().iter();

                if let Some(and_id) = children.next() {
                    if let Some(and_node) = arena.get_node(*and_id) {
                        let and_children = and_node.children();

                        write!(f, "      (")?;
                        and_node.kind().fmt_planning(f, interner)?; // imprime "and"

                        if and_children.is_empty() {
                            write!(f, " )")?; // cas vide => (and )
                        } else {
                            writeln!(f)?;
                            for child_id in and_children {
                                if let Some(ordering_node) = arena.get_node(*child_id) {
                                    write!(f, "        ")?;
                                    ordering_node.fmt_planning(f, arena, interner)?;
                                    writeln!(f)?;
                                } else {
                                    writeln!(f, "       <invalid>")?;
                                }
                            }
                            write!(f, "      )")?;
                        }
                    } else {
                        writeln!(f, "<invalid-and-node>")?;
                    }
                } else {
                    writeln!(f, "<no-children>")?;
                }

                Ok(())
            }

            AstKind::TaskOrderingConstraint => {
                write!(f, "(")?;
                // Affiche le content (= le "type" du lien, par ex. "before")
                self.content().fmt_planning(f, interner)?;

                let children = self.children();
                if children.len() == 2 {
                    // Premier fils
                    write!(f, " ")?;
                    if let Some(first_node) = arena.get_node(children[0]) {
                        first_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }

                    // Deuxième fils
                    write!(f, " ")?;
                    if let Some(second_node) = arena.get_node(children[1]) {
                        second_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }
                } else {
                    // Si pas exactement 2 enfants, on indique un problème
                    write!(f, "<unexpected-children>")?;
                }

                write!(f, ")")
            }

            AstKind::ActionDefBody | AstKind::MethodDefBody => {
                for child_id in self.children() {
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                    }
                }
                Ok(())
            }

            AstKind::ActionDef => {
                write!(f, "  (:action ")?;

                let mut children_iter = self.children().iter();

                // 1. Nom de l'action
                if let Some(name_id) = children_iter.next() {
                    if let Some(name_node) = arena.get_node(*name_id) {
                        name_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid_name>")?;
                    }
                } else {
                    write!(f, "<missing_name>")?;
                }

                // 2. Paramètres (optionnel)
                if let Some(params_id) = children_iter.next() {
                    if let Some(params_node) = arena.get_node(*params_id) {
                        write!(f, "\n    :parameters (")?;  // 4 espaces
                        let mut first = true;
                        for child_id in params_node.children() {
                            if !first {
                                write!(f, " ")?;
                            }
                            if let Some(child_node) = arena.get_node(*child_id) {
                                child_node.fmt_planning(f, arena, interner)?;
                            } else {
                                write!(f, "<invalid_param>")?;
                            }
                            first = false;
                        }
                        write!(f, ")")?;
                    }
                }

                // 3. Autres enfants (preconditions, effets, etc.)
                for rest_child_id in children_iter {
                    write!(f, "\n")?; // --> ici 4 espaces au lieu de 2
                    if let Some(rest_child_node) = arena.get_node(*rest_child_id) {
                        rest_child_node.fmt_planning(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid_child>")?;
                    }
                }

                write!(f, "  )")
            }


            AstKind::ObjectsDef => {
                write!(f, "  (:objects")?;
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                    }
                }
                write!(f, ")")
            }



            AstKind::Init => {
                writeln!(f, "  (:init")?;
                for child_id in self.children() {
                    write!(f, "  ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                        writeln!(f)?;
                    }
                }
                write!(f, ")")
            }

            AstKind::Goal => {
                writeln!(f, "  (:goal")?;
                for child_id in self.children() {
                    write!(f, "  ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                        writeln!(f)?;
                    }
                }
                write!(f, ")")
            }

            AstKind::Metric => {
                write!(f, "  (:metric")?;
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                    }
                }
                write!(f, ")")
            }





            AstKind::Constant
            | AstKind::Variable
            | AstKind::FunctionSymbol
            | AstKind::PrimitiveType
            | AstKind::DomainName
            | AstKind::ProblemName
            | AstKind::Number
            | AstKind::Predicate
            | AstKind::ActionSymbol
            | AstKind::MethodSymbol
            | AstKind::TaskSymbol
            | AstKind::PrefName
            | AstKind::Requirement
            | AstKind::TaskID
            | AstKind::Error => {
                self.content().fmt_planning(f, interner)
            }



            _ => {
                write!(f, "(DEFAULT{}", self.kind())?;
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning(f, arena, interner)?;
                    }
                }
                write!(f, ")")
            }
        }
    }

}
