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
        indent: usize,
    ) -> fmt::Result {
        for (i, child_id) in self.children().iter().enumerate() {
            if i > 0 && !multiline {
                write!(f, " ")?;
            }
            if let Some(child_node) = arena.get_node(*child_id) {
                if multiline {
                    // Indent seulement avec `indent` (pas indent+1)
                    // Pour que la première ligne soit indentée de 2 espaces, pas plus
                    let indent_str = Self::make_indent(indent);
                    write!(f, "{}", indent_str)?;
                }
                child_node.fmt_planning_syntax(f, arena, interner)?;
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
        indent: usize,
    ) -> fmt::Result {
        // Create indentation string based on the current indent level (2 spaces per level)
        let indent_str = "  ".repeat(indent);

        // If prefix is requested, write it with the current indentation
        if with_prefix {
            write!(f, "{}:task ", indent_str)?;
        }

        let children = self.children();

        if children.is_empty() {
            // If there are no children, print empty parentheses
            write!(f, "()")?;
        } else {
            // Otherwise, print opening parenthesis
            write!(f, "(")?;

            // Iterate over children and print each separated by a space
            for (i, child_id) in children.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }

                // Format each child node with the same indentation level
                if let Some(child_node) = arena.get_node(*child_id) {
                    child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                } else {
                    // If child node is invalid, print placeholder
                    write!(f, "<invalid>")?;
                }
            }

            // Close the parenthesis
            write!(f, ")")?;
        }

        // If prefix was written, write a newline; otherwise, just return Ok
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

    fn fmt_planning_syntax_with_indent(&self, f: &mut Formatter<'_>, arena: &TreeArena<Self>, interner: &StringInterner, indent: usize) -> fmt::Result {
        let indent_str = Self::make_indent(indent);


        match self.kind() {
            AstKind::Domain => {
                // Write the opening line with base indentation
                write!(f, "{}(define (domain ", indent_str)?;

                // Domain name (mandatory)
                if let Some(name_id) = self.children().get(0) {
                    if let Some(name_node) = arena.get_node(*name_id) {
                        name_node.fmt_planning_syntax_with_indent(f, arena, interner, 0)?;
                    } else {
                        write!(f, "<invalid-domain-name>")?;
                    }
                } else {
                    write!(f, "<missing-domain-name>")?;
                }

                write!(f, ")")?;

                // Other children (optional), starting from index 1
                for child_id in self.children().iter().skip(1) {
                    if let Some(child_node) = arena.get_node(*child_id) {
                        writeln!(f)?;
                        // Recurse with increased indentation
                        child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent + 1)?;
                    }
                }

                // Closing parenthesis of the domain
                writeln!(f)?;
                write!(f, "{})\n", indent_str)?;

                Ok(())
            }

            AstKind::Problem => {
                // Opening line with base indentation
                write!(f, "{}(define (problem ", indent_str)?;

                // Problem name (mandatory)
                if let Some(name_id) = self.children().get(0) {
                    if let Some(name_node) = arena.get_node(*name_id) {
                        name_node.fmt_planning_syntax(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid-problem-name>")?;
                    }
                } else {
                    write!(f, "<missing-problem-name>")?;
                }

                write!(f, ")")?;

                // Iterate over children starting from the second (index 1)
                for child_id in self.children().iter().skip(1) {
                    if let Some(child_node) = arena.get_node(*child_id) {
                        writeln!(f)?;
                        let child_indent = Self::make_indent(indent + 1);
                        write!(f, "{}", child_indent)?;
                        child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent + 1)?;
                    }
                }

                // Closing line with base indentation
                writeln!(f)?;
                write!(f, "{})\n", indent_str)?;

                Ok(())
            }


            AstKind::RequireDef => {
                // Indentation de la ligne d'ouverture
                f.write_str(&indent_str)?;
                write!(f, "(:requirements")?;

                // Chaque enfant est écrit sur la même ligne, séparé par un espace
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax(f, arena, interner)?;
                    }
                }

                // Fermeture sur la même ligne
                writeln!(f, ")")
            }

            AstKind::TypesDef => {
                f.write_str(&indent_str)?;
                write!(f, "(")?;
                self.kind().fmt_planning_syntax(f, interner)?;
                writeln!(f)?;

                if let Some(child_id) = self.children().first() {
                    if let Some(child_node) = arena.get_node(*child_id) {
                        // Pass indent + 1 to indent children 2 spaces more than the opening line
                        child_node.fmt_typed_list(f, arena, interner, true, indent + 1)?;
                    }
                } else {
                    // No children: optionally write a comment or just an empty indented line
                    let empty_indent = Self::make_indent(indent + 1);
                    writeln!(f, "{}; <missing-typed-list>", empty_indent)?;
                }

                f.write_str(&indent_str)?;  // root indentation for closing parenthesis
                writeln!(f, ")")
            }

            AstKind::TypedList => {
                self.fmt_typed_list(f, arena, interner, false, indent)
            }

            AstKind::TypedItemElements => {
                write!(f, "{}", indent_str)?;
                for (i, child_id) in self.children().iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                    }
                }
                Ok(())
            }

            AstKind::TypedItem => {
                let indent_str = Self::make_indent(indent);
                let children = self.children();
                let n = children.len();

                if n == 0 {
                    // Nothing to display if there are no children
                    return Ok(());
                }

                // Write indentation before starting the typed item line
                write!(f, "{}", indent_str)?;

                // All but the last child are typed elements
                for (i, child_id) in children.iter().take(n - 1).enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                    } else {
                        write!(f, "<invalid_node>")?;
                    }
                }

                // If there's more than one child, display " - " before the type node
                if n > 1 {
                    write!(f, " - ")?;
                    if let Some(type_node) = arena.get_node(children[n - 1]) {
                        type_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
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
                            // Write indentation before the single type
                            write!(f, "{}", indent_str)?;
                            // Recursively format the type content with the current indentation
                            ty_node.content().fmt_planning_syntax_with_indent(f, interner, indent)
                        } else {
                            write!(f, "{}<invalid_node>", indent_str)
                        }
                    }

                    _ => {
                        // Write indentation and opening either keyword
                        write!(f, "{}(either", indent_str)?;

                        for child_id in self.children() {
                            write!(f, " ")?;
                            if let Some(ty_node) = arena.get_node(*child_id) {
                                // Format each type content recursively, no extra indent here since on the same line
                                ty_node.content().fmt_planning_syntax_with_indent(f, interner, indent)?;
                            } else {
                                write!(f, "<invalid_node>")?;
                            }
                        }
                        write!(f, ")")
                    }
                }
            }

            AstKind::ConstantsDef => {
                // Write the opening line with current indentation
                writeln!(f, "{}(:constants", indent_str)?;

                // Format the constants list line by line with increased indentation for readability
                if let Some(child_id) = self.children().first() {
                    if let Some(child_node) = arena.get_node(*child_id) {
                        // Assuming fmt_typed_list now takes indent parameter to handle multiline indent
                        child_node.fmt_typed_list(f, arena, interner, true, indent + 1)?;
                    }
                }

                // Write the closing parenthesis aligned with the opening line
                writeln!(f, "{})", indent_str)
            }

            AstKind::PredicatesDef => {
                // Write the opening line with current indentation
                writeln!(f, "{}(:predicates", indent_str)?;

                // Iterate over all children and format each predicate with increased indentation
                for child_id in self.children() {
                    if let Some(child_node) = arena.get_node(*child_id) {
                        // Write indent for predicates' lines (one level deeper)
                        let child_indent_str = Self::make_indent(indent + 1);
                        write!(f, "{}", child_indent_str)?;
                        child_node.fmt_planning_syntax(f, arena, interner)?;
                        writeln!(f)?;
                    }
                }

                // Write the closing parenthesis aligned with the opening line
                writeln!(f, "{})", indent_str)
            }


            AstKind::FunctionsDef => {
                // Write the opening line with current indentation
                write!(f, "{}(:functions", indent_str)?;

                // Iterate over children and format each function inline separated by spaces
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax(f, arena, interner)?;
                    }
                }

                // Write the closing parenthesis aligned with the opening line
                write!(f, ")")
            }

            AstKind::AtomicFormulaSkeleton
            | AstKind::AtomicFunctionSkeleton => {
                // Write the opening parenthesis with the given indentation
                write!(f, "{}(", indent_str)?;

                let children = self.children();

                for (i, child_id) in children.iter().enumerate() {
                    if let Some(child_node) = arena.get_node(*child_id) {
                        // Write a space before this child only if:
                        // - it's not the first child
                        // - AND the child node's typed list is not empty (i.e., it has children)
                        if i > 0 && !child_node.children().is_empty() {
                            write!(f, " ")?;
                        }
                        // Format the child node recursively
                        child_node.fmt_planning_syntax(f, arena, interner)?;
                    } else {
                        // For invalid child nodes, write a space before it if not the first child
                        if i > 0 {
                            write!(f, " ")?;
                        }
                        // Write an error placeholder for invalid nodes
                        write!(f, "<invalid>")?;
                    }
                }

                // Write the closing parenthesis without any extra space
                write!(f, ")")
            }


            AstKind::TaskDef => {
                // Write opening line with indentation
                write!(f, "{}(:task ", indent_str)?;

                let children = self.children();
                let mut idx = 0;

                // 1. The task name (mandatory)
                if let Some(&name_id) = children.get(idx) {
                    if let Some(name_node) = arena.get_node(name_id) {
                        name_node.fmt_planning_syntax(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid_name>")?;
                    }
                } else {
                    write!(f, "<missing_name>")?;
                }
                idx += 1;

                // 2. The parameters (mandatory)
                if let Some(&params_id) = children.get(idx) {
                    if let Some(params_node) = arena.get_node(params_id) {
                        writeln!(f)?;
                        write!(f, "{}:parameters (", Self::make_indent(indent + 1))?;

                        let mut first = true;
                        for child_id in params_node.children() {
                            if !first {
                                write!(f, " ")?;
                            }
                            if let Some(child_node) = arena.get_node(*child_id) {
                                child_node.fmt_planning_syntax(f, arena, interner)?;
                            } else {
                                write!(f, "<invalid-parameter>")?;
                            }
                            first = false;
                        }

                        write!(f, ")")?;
                    } else {
                        writeln!(f)?;
                        write!(f, "{}<invalid_parameters>", Self::make_indent(indent + 1))?;
                    }
                } else {
                    // parameters node is missing: error
                    writeln!(f)?;
                    write!(f, "{}<missing_parameters>", Self::make_indent(indent + 1))?;
                }

                // Close the task definition
                writeln!(f, "\n{})", indent_str)
            }


            AstKind::MethodDef => {
                let children = self.children();

                // Base indentation for :parameters etc.
                let indent_param = Self::make_indent(indent + 1);

                // Write the opening line with base indentation
                write!(f, "{}(:method ", indent_str)?;

                // === 1. Name (mandatory) ===
                if let Some(&name_id) = children.get(0) {
                    if let Some(name_node) = arena.get_node(name_id) {
                        name_node.fmt_planning_syntax(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid-method-name>")?;
                    }
                } else {
                    write!(f, "<missing-method-name>")?;
                }

                // === 2. Parameters (mandatory) ===
                if let Some(&params_id) = children.get(1) {
                    if let Some(params_node) = arena.get_node(params_id) {
                        writeln!(f)?;
                        write!(f, "{}:parameters (", indent_param)?;
                        params_node.fmt_typed_list(f, arena, interner, false, indent + 1)?;
                        write!(f, ")")?;
                    } else {
                        writeln!(f)?;
                        write!(f, "{}<invalid-parameters>", indent_param)?;
                    }
                } else {
                    writeln!(f)?;
                    write!(f, "{}<missing-parameters>", indent_param)?;
                }

                // === 3. MethodDefBody (mandatory) ===
                if let Some(&body_id) = children.get(2) {
                    writeln!(f)?;
                    if let Some(body_node) = arena.get_node(body_id) {
                        body_node.fmt_planning_syntax_with_indent(f, arena, interner, indent + 1)?;
                    } else {
                        let indent_body = Self::make_indent(indent + 1);
                        write!(f, "{}<invalid-method-body>", indent_body)?;
                    }
                } else {
                    writeln!(f)?;
                    let indent_body = Self::make_indent(indent + 1);
                    write!(f, "{}<missing-method-body>", indent_body)?;
                }

                // Closing parenthesis
                writeln!(f, "{})", indent_str)?;

                Ok(())
            }

            AstKind::Task => {
                // Call fmt_task with increased indentation level for nested formatting
                self.fmt_task(f, arena, interner, true, 0)
            }


            AstKind::PreconditionDef
            | AstKind::EffectDef
            | AstKind::MethodPreconditionDef
            | AstKind::TaskLogicalConstraintDef => {
                // Write the kind label
                write!(f, "{}", indent_str)?;
                self.kind().fmt_planning_syntax(f, interner)?;

                // Newline after the label
                writeln!(f)?;

                let child_indent_str = Self::make_indent(indent + 1);

                // Write the indentation for the child node
                write!(f, "{}", child_indent_str)?;

                // Format the first child if it exists, else print <no-children>
                if let Some(first_child_id) = self.children().first() {
                    if let Some(first_child_node) = arena.get_node(*first_child_id) {
                        first_child_node.fmt_planning_syntax(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }
                } else {
                    // No children: print <no-children>
                    write!(f, "<no-children>")?;
                }
                Ok(())
            }


            AstKind::AtomicFormula
            | AstKind::FunctionTerm => {
                // Write the opening parenthesis with current indentation
                write!(f, "{}(", indent_str)?;

                let mut first = true;

                // Iterate over all children and format them with a space separator
                for child_id in self.children() {
                    if !first {
                        write!(f, " ")?;
                    }
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }
                    first = false;
                }

                // Write the closing parenthesis
                write!(f, ")")
            }


            AstKind::Assign
            | AstKind::FComp => {
                // Write the opening parenthesis with current indentation
                write!(f, "{}(", indent_str)?;

                // Write the operator keyword using planning syntax formatting
                write!(f, "{}", self.content().to_string())?;

                // Format each child node, separated by spaces
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }
                }

                // Write the closing parenthesis
                write!(f, ")")
            }

            AstKind::And
            | AstKind::Or
            | AstKind::Not
            | AstKind::Imply
            | AstKind::AtStart
            | AstKind::AtEnd
            | AstKind::Overall => {
                // Write the opening parenthesis with current indentation
                write!(f, "{}(", indent_str)?;

                // Write the operator keyword using planning syntax formatting
                self.kind().fmt_planning_syntax(f, interner)?;

                // Format each child node, separated by spaces
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }
                }

                // Write the closing parenthesis
                write!(f, ")")
            }



            AstKind::TaskNetworkDef => {
                let children = self.children();

                for (i, &child_id) in children.iter().enumerate() {
                    let is_last = i == children.len() - 1;

                    if let Some(child_node) = arena.get_node(child_id) {
                        match child_node.kind() {
                            AstKind::OrderedSubtaskDef
                            | AstKind::PartiallyOrderedSubtaskDef => {
                                child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                                if !is_last {
                                    writeln!(f)?;
                                }
                            }
                            AstKind::TaskOrderingConstraintDef => {
                                child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                                if !is_last {
                                    writeln!(f)?;
                                }
                            }
                            AstKind::TaskLogicalConstraintDef => {
                                child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                                if !is_last {
                                    writeln!(f)?;
                                }
                            }
                            _ => {
                                writeln!(f, "{}<unexpected-child-kind>", indent_str)?;
                            }
                        }
                    } else {
                        writeln!(f, "{}<invalid-child-node>", indent_str)?;
                    }
                }

                Ok(())
            }



            AstKind::OrderedSubtaskDef
            | AstKind::PartiallyOrderedSubtaskDef => {
                // Write the type of subtask with current indentation
                write!(f, "{}", indent_str)?;
                self.kind().fmt_planning_syntax(f, interner)?;
                writeln!(f)?;

                let mut children = self.children().iter();

                // The first child is expected to be the logical AND (or similar operator)
                if let Some(and_id) = children.next() {
                    if let Some(and_node) = arena.get_node(*and_id) {
                        let and_children = and_node.children();

                        // Begin the clause with increased indentation: (and
                        write!(f, "{}(", Self::make_indent(indent + 1))?;
                        and_node.kind().fmt_planning_syntax(f, interner)?; // prints "and"

                        if and_children.is_empty() {
                            // Empty case, close the clause on the same line
                            write!(f, ")")?;
                        } else {
                            writeln!(f)?;
                            for child_id in and_children {
                                if let Some(task_node) = arena.get_node(*child_id) {
                                    match task_node.kind() {
                                        AstKind::Task => {
                                            // Print task without prefix with one more indentation level
                                            write!(f, "{}", Self::make_indent(indent + 2))?;
                                            task_node.fmt_task(f, arena, interner, false, 0)?;
                                            writeln!(f)?;
                                        }
                                        AstKind::TaggedTask => {
                                            // Print tagged task using fmt_planning with indentation
                                            write!(f, "{}", Self::make_indent(indent + 2))?;
                                            task_node.fmt_planning_syntax(f, arena, interner)?;
                                            writeln!(f)?;
                                        }
                                        other => {
                                            // Unexpected node kind
                                            writeln!(f, "{}<unexpected-{}>", Self::make_indent(indent + 2), other)?;
                                        }
                                    }
                                } else {
                                    writeln!(f, "{}<invalid>", Self::make_indent(indent + 1))?;
                                }
                            }
                            // Close the clause with one level less indentation
                            write!(f, "{})", Self::make_indent(indent + 1))?;
                        }
                    } else {
                        writeln!(f, "{}<invalid-and-node>", indent_str)?;
                    }
                } else {
                    writeln!(f, "{}<no-children>", indent_str)?;
                }

                Ok(())
            }

            AstKind::TaggedTask => {
                let children = self.children();

                // Validate that there are exactly 2 children for TaggedTask
                if children.len() != 2 {
                    // Print invalid placeholder with current indentation
                    write!(f, "{}<invalid-tagged-task>", indent_str)?;
                    return Ok(());
                }

                // Write opening parenthesis with current indentation
                write!(f, "{}(", indent_str)?;

                // Print the TaskID (first child)
                if let Some(task_id_node) = arena.get_node(children[0]) {
                    task_id_node.fmt_planning_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid-task-id>")?;
                }

                write!(f, " ")?;

                // Print the actual task (second child) without ":task" prefix
                if let Some(task_node) = arena.get_node(children[1]) {
                    task_node.fmt_task(f, arena, interner, false, 0)?;
                } else {
                    write!(f, "<invalid-task>")?;
                }

                // Close the parenthesis
                write!(f, ")")
            }

            AstKind::TaskOrderingConstraintDef => {
                // Write the kind line with current indentation
                write!(f, "{}", indent_str)?;
                self.kind().fmt_planning_syntax(f, interner)?;
                writeln!(f)?;

                let mut children = self.children().iter();

                if let Some(and_id) = children.next() {
                    if let Some(and_node) = arena.get_node(*and_id) {
                        let and_children = and_node.children();

                        // Write opening line for the 'and' clause with increased indentation
                        write!(f, "{}(", Self::make_indent(indent + 1))?;
                        and_node.kind().fmt_planning_syntax(f, interner)?; // prints "and"

                        if and_children.is_empty() {
                            write!(f, ")")?;
                        } else {
                            writeln!(f)?;
                            for child_id in and_children {
                                if let Some(ordering_node) = arena.get_node(*child_id) {
                                    // Print each ordering child with further indentation
                                    write!(f, "{}", Self::make_indent(indent + 2))?;
                                    ordering_node.fmt_planning_syntax(f, arena, interner)?;
                                    writeln!(f)?;
                                } else {
                                    writeln!(f, "{}<invalid>", Self::make_indent(indent + 2))?;
                                }
                            }
                            // Closing parenthesis with increased indentation
                            write!(f, "{})", Self::make_indent(indent + 1))?;
                        }
                    } else {
                        writeln!(f, "{}<invalid-and>", indent_str)?;
                    }
                } else {
                    writeln!(f, "{}<no-children>", indent_str)?;
                }

                Ok(())
            }

            AstKind::TaskOrderingConstraint => {
                let indent_str = Self::make_indent(indent); // Compute indent string once

                // Write opening parenthesis with indentation
                write!(f, "{}(", indent_str)?;

                // Print the content (e.g., "<")
                self.content().fmt_planning_syntax(f, interner)?;

                let children = self.children();
                if children.len() == 2 {
                    // Print first child with a space before
                    write!(f, " ")?;
                    if let Some(first_node) = arena.get_node(children[0]) {
                        first_node.fmt_planning_syntax(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }

                    // Print second child with a space before
                    write!(f, " ")?;
                    if let Some(second_node) = arena.get_node(children[1]) {
                        second_node.fmt_planning_syntax(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }
                } else {
                    // If not exactly 2 children, print an error placeholder
                    write!(f, "<unexpected-children>")?;
                }

                // Close the parenthesis without additional indent (since it closes the current line)
                write!(f, ")")
            }

            AstKind::MethodDefBody => {
                let children = self.children();

                // Task (toujours présent en premier)
                if let Some(&task_id) = children.get(0) {
                    write!(f, "{}", indent_str)?;
                    if let Some(task_node) = arena.get_node(task_id) {
                        task_node.fmt_task(f, arena, interner, true, 0)?;
                    } else {
                        write!(f, "<invalid-task>")?;
                    }
                } else {
                    writeln!(f)?;
                    write!(f, "{}<missing-task>", indent_str)?;
                }

                // Preconditions (optionnel, uniquement s'il y a 3 enfants)
                if children.len() == 3 {
                    if let Some(&precond_id) = children.get(1) {
                        if let Some(precond_node) = arena.get_node(precond_id) {
                            precond_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                            writeln!(f)?;
                        }
                    }
                }

                // Task Network (toujours le dernier enfant)
                if let Some(&task_network_id) = children.last() {
                    if let Some(task_network_node) = arena.get_node(task_network_id) {
                        task_network_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                        writeln!(f)?;
                    } else {
                        write!(f, "<invalid-task-network>")?;
                    }
                } else {
                    writeln!(f)?;
                    write!(f, "{}<missing-task-network>", indent_str)?;
                }

                Ok(())
            }


            AstKind::ActionDef => {
                let children = self.children();

                // Base indentation for :parameters etc.
                let indent_param = Self::make_indent(indent + 1);

                // Write the opening line with base indentation
                write!(f, "{}(:action ", indent_str)?;

                // === 1. Name (mandatory) ===
                if let Some(&name_id) = children.get(0) {
                    if let Some(name_node) = arena.get_node(name_id) {
                        name_node.fmt_planning_syntax(f, arena, interner)?;
                    } else {
                        write!(f, "<invalid-action-name>")?;
                    }
                } else {
                    write!(f, "<missing-action-name>")?;
                }

                // === 2. Parameters (mandatory) ===
                if let Some(&params_id) = children.get(1) {
                    if let Some(params_node) = arena.get_node(params_id) {
                        writeln!(f)?;
                        write!(f, "{}:parameters (", indent_param)?;
                        params_node.fmt_typed_list(f, arena, interner, false, indent + 1)?;
                        write!(f, ")")?;
                    } else {
                        writeln!(f)?;
                        write!(f, "{}<invalid-parameters>", indent_param)?;
                    }
                } else {
                    writeln!(f)?;
                    write!(f, "{}<missing-parameters>", indent_param)?;
                }

                // === 3. ActionDefBody (mandatory) ===
                if let Some(&body_id) = children.get(2) {
                    writeln!(f)?;
                    if let Some(body_node) = arena.get_node(body_id) {
                        body_node.fmt_planning_syntax_with_indent(f, arena, interner, indent + 1)?;
                        writeln!(f)?;
                    } else {
                        let indent_body = Self::make_indent(indent + 1);
                        write!(f, "{}<invalid-action-body>", indent_body)?;
                        writeln!(f)?;
                    }
                } else {
                    writeln!(f)?;
                    let indent_body = Self::make_indent(indent + 1);
                    write!(f, "{}<missing-action-body>", indent_body)?;
                }

                // Closing parenthesis
                writeln!(f, "{})", indent_str)?;

                Ok(())
            }

            AstKind::ActionDefBody => {
                let children = self.children();
                let indent_child = Self::make_indent(indent);

                let mut idx = 0;

                // 1. PreconditionDef (optional)
                if let Some(&child_id) = children.get(idx) {
                    if let Some(child_node) = arena.get_node(child_id) {
                        child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                        writeln!(f)?;
                    } else {
                        writeln!(f)?;
                        write!(f, "{}<invalid-precondition>", indent_child)?;
                    }
                    idx += 1;
                }

                // 2. EffectDef (optional)
                if let Some(&child_id) = children.get(idx) {
                    if let Some(child_node) = arena.get_node(child_id) {
                        child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                    } else {
                        write!(f, "{}<invalid-effect>", indent_child)?;
                    }
                    idx += 1;
                }

                Ok(())
            }

            AstKind::ObjectsDef => {
                // Write the opening line with current indentation
                write!(f, "{}(:objects", Self::make_indent(indent))?;

                // Print children separated by spaces
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                    } else {
                        write!(f, "<invalid>")?;
                    }
                }

                // Close with current indentation (same line)
                write!(f, ")")
            }

            AstKind::Init => { // indentation pour la ligne (:init
                let child_indent_str = Self::make_indent(indent + 1); // indentation pour les enfants

                // Write the opening line for the init section with proper indent
                writeln!(f, "{}(:init", indent_str)?;

                // Iterate over all children and format them with increased indentation
                for child_id in self.children() {
                    write!(f, "{}", child_indent_str)?; // indentation enfant
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent + 1)?;
                        writeln!(f)?; // newline after each child
                    }
                }

                // Write the closing parenthesis with base indentation
                writeln!(f, "{})", indent_str)
            }

            AstKind::Goal => {
                let child_indent_str = Self::make_indent(indent + 1); // indentation pour les enfants

                // Write the opening line for the goal section with proper indentation
                writeln!(f, "{}(:goal", indent_str)?;

                // Iterate over all children and format them with increased indentation
                for child_id in self.children() {
                    write!(f, "{}", child_indent_str)?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent + 1)?;
                        writeln!(f)?; // newline after each child
                    }
                }

                // Write the closing parenthesis with base indentation
                writeln!(f, "{})", indent_str)
            }


            AstKind::Metric => {

                // Write the opening line for the metric section with base indentation
                write!(f, "{}(:metric", indent_str)?;

                // Iterate over all children, separated by spaces (no newlines)
                for child_id in self.children() {
                    write!(f, " ")?; // space before each child
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent + 1)?;
                    }
                }

                // Write the closing parenthesis with no extra indentation (same line)
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
            | AstKind::TaskID => {
                write!(f, "{}", indent_str)?;
                self.content().fmt_planning_syntax_with_indent(f, interner, indent)
            }

            | AstKind::Error => {
                write!(f, "{}<error>", indent_str)
            }


                _ => {
                write!(f, "(DEFAULT{}", self.kind())?;
                for child_id in self.children() {
                    write!(f, " ")?;
                    if let Some(child_node) = arena.get_node(*child_id) {
                        child_node.fmt_planning_syntax_with_indent(f, arena, interner, indent)?;
                    }
                }
                write!(f, ")")
            }
        }
    }

}
