use crate::aiplan4rust::syntax::Span;
use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::syntax::int_ast::{AstContent, IntAstKind};

/// Represents a node in the Abstract Syntax Tree (AST).
///
/// This struct models a syntactic element of a program or a PDDL specification.
/// Each node contains:
/// - a type (`AstKind`) indicating its syntactic category,
/// - a list of child nodes,
/// - a span (`Span`) representing its position in the source file,
/// - and a unique identifier that can be assigned.
///
/// # Derived Traits
/// - `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`
///
/// # Example
/// ```rust
/// use parser::Node;
/// use parser::AstKind;
///
/// let node = Node::new(AstKind::Domain, vec![], 0, 10);
/// println!("Created node: {:?}", node);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Node {
    kind: IntAstKind,
    content: AstContent,
    children: Vec<Box<Node>>,
    span: Span,
}

impl Node {

    /// Creates a new AST node with the specified type, children, and source positions.
    ///
    /// # Arguments
    /// - `kind`: The node type (`AstKind`).
    /// - `children`: Vector of child nodes (`Box<Node>`).
    /// - `start`: Start position in the character stream (offset).
    /// - `end`: End position in the character stream (offset).
    ///
    /// # Returns
    /// A new `Node` instance.
    ///
    /// # Example
    /// ```rust
    /// let node = Node::new(AstKind::PrimitiveType, vec![], 0, 10);
    /// ```
    pub fn new(kind: IntAstKind, content: AstContent, children: Vec<Box<Node>>, start: usize, end: usize) -> Node {
        Node {
            kind,
            content,
            children,
            span: Span::new(start, end),
        }
    }

    /// Creates a new AST node with an explicit `Span`.
    ///
    /// # Arguments
    /// - `kind`: The node type.
    /// - `children`: Child nodes.
    /// - `span`: Source interval (`Span`).
    ///
    /// # Returns
    /// A new `Node` instance.
    pub fn new_with_span(kind: IntAstKind, content: AstContent, children: Vec<Box<Node>>, span: Span) -> Node {
        Node {
            kind,
            content,
            children,
            span,
        }
    }


    // === Accessors & Mutators ===

    /// Returns the total size of the subtree (number of nodes).
    ///
    /// # Example
    /// ```rust
    /// let size = node.size();
    /// ```
    pub fn size(&self) -> usize {
        1 + self.children.iter().map(|c| c.size()).sum::<usize>()
    }


    /// Returns a reference to the node's type (`AstKind`).
    pub fn kind(&self) -> &IntAstKind {
        &self.kind
    }

    pub fn kind_mut(&mut self) -> &mut IntAstKind {
        &mut self.kind
    }

    pub fn set_kind(&mut self, new_kind: IntAstKind) {
        self.kind = new_kind;
    }

    /// Returns an immutable reference to the child nodes.
    pub fn children(&self) -> &Vec<Box<Node>> {
        &self.children
    }

    /// Returns a mutable reference to the child nodes.
    pub fn children_mut(&mut self) -> &mut Vec<Box<Node>> {
        &mut self.children
    }

    /// Replaces the child nodes with a new vector.
    pub fn set_children(&mut self, new_children: Vec<Box<Node>>) {
        self.children = new_children;
    }

    /// Returns a reference to the node's span.
    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    /// Returns the start offset in the character stream.
    pub fn start_offset(&self) -> usize {
        self.span.start()
    }

    /// Returns the end offset in the character stream.
    pub fn end_offset(&self) -> usize {
        self.span.end()
    }

    /// Returns the start position as (line, column).
    ///
    /// Returns `(usize::MAX, usize::MAX)` if not initialized.
    pub fn start_position(&self) -> (usize, usize) {
        self.span.start_position()
    }

    /// Returns the end position as (line, column).
    ///
    /// Returns `(usize::MAX, usize::MAX)` if not initialized.
    pub fn end_position(&self) -> (usize, usize) {
        self.span.end_position()
    }

    /// Sets the start position (line, column).
    ///
    /// # Arguments
    /// - `line`: line number.
    /// - `column`: column number.
    pub fn set_start_position(&mut self, line: usize, column: usize) {
        self.span.set_begin_line(line);
        self.span.set_begin_column(column);
    }

    /// Sets the end position (line, column).
    ///
    /// # Arguments
    /// - `line`: line number.
    /// - `column`: column number.
    pub fn set_end_position(&mut self, line: usize, column: usize) {
        self.span.set_end_line(line);
        self.span.set_end_column(column);
    }

}
