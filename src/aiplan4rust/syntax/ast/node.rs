use crate::aiplan4rust::syntax::{Span, StringInterner};
use std::fmt;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};

/// Represents a node in the Abstract Syntax Tree (AST).
///
/// Each `Node` corresponds to a syntactic element in a program or PDDL specification.
/// It contains:
/// - `kind`: the syntactic category of this node (`AstKind`),
/// - `content`: additional content or value related to the node (`AstContent`),
/// - `children`: a vector of child nodes, allowing tree structure representation,
/// - `span`: source code position (`Span`) indicating where this node appears.
///
/// This structure supports hierarchical representation of source code or domain-specific
/// languages, enabling traversal, analysis, and transformation of the syntax.
///
/// # Traits Derived
/// - `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash` for usability in collections and debugging.
/// - `Default` for default initialization.
/// - `Serialize` and `Deserialize` for (de)serialization support.
///
/// # Example
/// ```rust
/// use parser::Node;
/// use parser::AstKind;
///
/// let node = Node::new(AstKind::Domain, AstContent::None, vec![], 0, 10);
/// println!("Created node: {:?}", node);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Node {
    kind: AstKind,
    content: AstContent,
    children: Vec<Box<Node>>,
    span: Span,
}

impl Node {
    /// Constructs a new `Node` with given kind, content, children, and source offsets.
    ///
    /// # Arguments
    /// * `kind` - The type/category of this node (`AstKind`).
    /// * `content` - Additional content or value associated with this node (`AstContent`).
    /// * `children` - Vector of boxed child nodes.
    /// * `start` - Start offset (byte or character) in the source code.
    /// * `end` - End offset in the source code.
    ///
    /// # Returns
    /// A newly created `Node` instance.
    ///
    /// # Example
    /// ```rust
    /// let node = Node::new(AstKind::PrimitiveType, AstContent::None, vec![], 0, 10);
    /// ```
    pub fn new(kind: AstKind, content: AstContent, children: Vec<Box<Node>>, start: usize, end: usize) -> Node {
        Node {
            kind,
            content,
            children,
            span: Span::new(start, end),
        }
    }

    /// Constructs a new `Node` with given kind, content, children, and explicit `Span`.
    ///
    /// # Arguments
    /// * `kind` - The node kind.
    /// * `content` - Associated content.
    /// * `children` - Child nodes.
    /// * `span` - Source code interval as a `Span` struct.
    ///
    /// # Returns
    /// A newly created `Node`.
    pub fn new_with_span(kind: AstKind, content: AstContent, children: Vec<Box<Node>>, span: Span) -> Node {
        Node {
            kind,
            content,
            children,
            span,
        }
    }

    // === Accessors and Mutators ===

    /// Returns the total number of nodes in this subtree (including self).
    ///
    /// # Example
    /// ```rust
    /// let size = node.size();
    /// ```
    pub fn size(&self) -> usize {
        1 + self.children.iter().map(|c| c.size()).sum::<usize>()
    }

    /// Returns a reference to this node's kind (`AstKind`).
    pub fn kind(&self) -> &AstKind {
        &self.kind
    }

    /// Returns a mutable reference to this node's kind (`AstKind`).
    pub fn kind_mut(&mut self) -> &mut AstKind {
        &mut self.kind
    }

    /// Sets the node kind.
    ///
    /// # Arguments
    /// * `new_kind` - New kind to set.
    pub fn set_kind(&mut self, new_kind: AstKind) {
        self.kind = new_kind;
    }

    /// Returns a reference to the node's content (`AstContent`).
    pub fn content(&self) -> &AstContent {
        &self.content
    }

    /// Returns a mutable reference to the node's content.
    pub fn content_mut(&mut self) -> &mut AstContent {
        &mut self.content
    }

    /// Sets the node's content.
    ///
    /// # Arguments
    /// * `new_content` - New content to assign.
    pub fn set_content(&mut self, new_content: AstContent) {
        self.content = new_content;
    }

    /// Returns a reference to the vector of child nodes.
    pub fn children(&self) -> &Vec<Box<Node>> {
        &self.children
    }

    /// Returns a mutable reference to the child nodes.
    pub fn children_mut(&mut self) -> &mut Vec<Box<Node>> {
        &mut self.children
    }

    /// Replaces the current child nodes with a new vector.
    ///
    /// # Arguments
    /// * `new_children` - New vector of child nodes.
    pub fn set_children(&mut self, new_children: Vec<Box<Node>>) {
        self.children = new_children;
    }

    /// Returns a reference to the node's source code span.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns a mutable reference to the node's span.
    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    /// Returns the start offset in the source code.
    pub fn start_offset(&self) -> usize {
        self.span.start()
    }

    /// Returns the end offset in the source code.
    pub fn end_offset(&self) -> usize {
        self.span.end()
    }

    /// Returns the start position as a (line, column) tuple.
    ///
    /// Returns `(usize::MAX, usize::MAX)` if uninitialized.
    pub fn start_position(&self) -> (usize, usize) {
        self.span.start_position()
    }

    /// Returns the end position as a (line, column) tuple.
    ///
    /// Returns `(usize::MAX, usize::MAX)` if uninitialized.
    pub fn end_position(&self) -> (usize, usize) {
        self.span.end_position()
    }

    /// Sets the start position (line and column).
    ///
    /// # Arguments
    /// * `line` - Line number.
    /// * `column` - Column number.
    pub fn set_start_position(&mut self, line: usize, column: usize) {
        self.span.set_begin_line(line);
        self.span.set_begin_column(column);
    }

    /// Sets the end position (line and column).
    ///
    /// # Arguments
    /// * `line` - Line number.
    /// * `column` - Column number.
    pub fn set_end_position(&mut self, line: usize, column: usize) {
        self.span.set_end_line(line);
        self.span.set_end_column(column);
    }

    /// Formats the node as a string with indentation for readability,
    /// but without any external context for resolving identifiers.
    ///
    /// # Arguments
    /// * `f` - Formatter to write to.
    /// * `indent` - Current indentation level (number of indent steps).
    ///
    /// # Returns
    /// `fmt::Result` indicating success or failure.
    pub fn fmt_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        let mut idx: Option<&mut usize> = None;
        self.fmt_internal(f, None, indent, &mut idx)
    }

    /// Formats the node using an external `StringInterner` context to resolve identifiers.
    ///
    /// # Arguments
    /// * `f` - Formatter to write to.
    /// * `ctx` - Reference to a `StringInterner` for identifier resolution.
    ///
    /// # Returns
    /// `fmt::Result`.
    pub fn fmt_with_context(&self, f: &mut fmt::Formatter<'_>, ctx: &StringInterner) -> fmt::Result {
        let mut idx: Option<&mut usize> = None;
        self.fmt_internal(f, Some(ctx), 0, &mut idx)
    }

    /// Formats the node with context, indentation, and index tracking.
    ///
    /// # Arguments
    /// * `f` - Formatter to write to.
    /// * `ctx` - Optional context to resolve identifiers.
    /// * `indent` - Indentation level.
    /// * `index` - Mutable reference to a counter index.
    ///
    /// # Returns
    /// `fmt::Result`.
    pub fn fmt_with_context_and_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        ctx: &StringInterner,
        indent: usize,
        index: &mut usize,
    ) -> fmt::Result {
        let mut index_wrapper = Some(index);
        self.fmt_internal(f, Some(ctx), indent, &mut index_wrapper)
    }

    /// Internal helper for formatted output.
    ///
    /// Prints the node recursively with indentation, optional context for
    /// resolving identifiers, and optional indexing.
    ///
    /// # Arguments
    /// * `f` - Formatter.
    /// * `ctx` - Optional context for resolving identifiers.
    /// * `indent` - Current indentation level.
    /// * `index` - Optional mutable reference to index counter.
    ///
    /// # Returns
    /// `fmt::Result`.
    fn fmt_internal(
        &self,
        f: &mut fmt::Formatter<'_>,
        ctx: Option<&StringInterner>,
        indent: usize,
        index: &mut Option<&mut usize>,
    ) -> fmt::Result {
        let indent_str = "  ".repeat(indent);

        // Resolve content string either from context or show raw content
        let content_str = match &self.content {
            AstContent::Ident(idx) => match ctx {
                Some(c) => c.get_str(*idx).unwrap_or("(unknown)").to_string(),
                None => format!("Ident({})", idx),
            },
            _ => format!("{:?}", self.content),
        };

        // Print with or without index
        if let Some(idx_ref) = index.as_deref_mut() {
            writeln!(
                f,
                "{}[{}] kind: {:?}, content: {}, span: {:?}",
                indent_str,
                *idx_ref,
                self.kind,
                content_str,
                self.span
            )?;
            *idx_ref += 1;
        } else {
            writeln!(
                f,
                "{}kind: {:?}, content: {}, span: {:?}",
                indent_str,
                self.kind,
                content_str,
                self.span
            )?;
        }

        // Recursively print children
        for child in &self.children {
            child.fmt_internal(f, ctx, indent + 1, index)?;
        }

        Ok(())
    }
}

impl Display for Node {
    /// Implements `Display` trait for pretty printing the node with indentation.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}
