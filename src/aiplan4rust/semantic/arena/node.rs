use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::syntax::{AstKindOld, Span};

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
/// - `kind`: The type of AST node (e.g., expression, statement).
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
pub struct Node {
    kind: AstKindOld,
    children: Vec<usize>,
    span: Span,
    parent: Option<usize>,
}

impl Node {
    /// Creates a new `Node` with the given kind, span, and optional parent.
    ///
    /// The node initially has no children.
    ///
    /// # Arguments
    ///
    /// * `kind` - The syntax kind of this node.
    /// * `span` - The source code span covered by this node.
    /// * `parent` - The optional index of the parent node. Use `None` if root.
    ///
    /// # Returns
    ///
    /// A newly created `Node`.
    pub fn new(kind: AstKindOld, span: Span, parent: Option<usize>) -> Self {
        Node {
            kind,
            children: Vec::new(),
            span,
            parent,
        }
    }

    /// Returns a reference to the kind of this node.
    pub fn kind(&self) -> &AstKindOld {
        &self.kind
    }

    /// Returns a slice of indices referring to this node’s children.
    pub fn children(&self) -> &[usize] {
        &self.children
    }

    /// Returns a reference to the source code span of this node.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns the optional index of this node’s parent.
    ///
    /// Returns `None` if this node has no parent (i.e., it is a root node).
    pub fn parent(&self) -> Option<usize> {
        self.parent
    }

    /// Returns `true` if this node is a root node (has no parent).
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Sets the kind of this node.
    ///
    /// # Arguments
    ///
    /// * `kind` - The new kind to assign to the node.
    pub fn set_kind(&mut self, kind: AstKindOld) {
        self.kind = kind;
    }

    /// Sets the source code span of this node.
    ///
    /// # Arguments
    ///
    /// * `span` - The new span to assign to the node.
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }

    /// Sets the parent of this node.
    ///
    /// This is a crate-private method intended to be used by arena internals.
    ///
    /// # Arguments
    ///
    /// * `parent` - The optional index of the parent node.
    pub(crate) fn set_parent(&mut self, parent: Option<usize>) {
        self.parent = parent;
    }

    /// Adds a child node index to this node’s children.
    ///
    /// This is a crate-private method intended to be used by arena internals.
    ///
    /// # Arguments
    ///
    /// * `child_id` - The index of the child node to add.
    pub(crate) fn add_child(&mut self, child_id: usize) {
        self.children.push(child_id);
    }
}

impl fmt::Display for Node {
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
        let children_str = self.children
            .iter()
            .map(|idx| idx.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "Node(kind={:?}, span={:?}, parent={:?}, children=[{}])",
            self.kind, self.span, self.parent, children_str
        )
    }
}
