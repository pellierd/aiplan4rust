use std::fmt;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::syntax::elements::{ArithmeticOp, AssignOp, BinaryComp, Ident, Optimization, Requirement};

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
    kind: AstKind,
    content: AstContent,
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
    pub fn new(kind: AstKind, content: AstContent, span: Span, parent: Option<usize>) -> Self {
        Node {
            kind,
            content,
            children: Vec::new(),
            span,
            parent,
        }
    }

    /// Returns the total number of nodes in this subtree, including the current node.
    ///
    /// This is computed recursively as:
    /// `1 + sum(size of each child)`.
    ///
    /// # Example
    /// ```
    /// let size = node.size();
    /// ```
    pub fn size(&self, arena: &[Node]) -> usize {
        let mut count = 0;
        let mut stack = vec![self];

        while let Some(node) = stack.pop() {
            count += 1;
            for &child_idx in &node.children {
                stack.push(&arena[child_idx]);
            }
        }

        count
    }

    /// Returns the number of direct children of this node.
    ///
    /// # Returns
    /// * `usize` - The number of immediate child nodes.
    pub fn arity(&self) -> usize {
        self.children.len()
    }

    /// Returns the depth of the subtree rooted at this node.
    ///
    /// The depth is defined as:
    /// - `1` if the node is a leaf (has no children),
    /// - otherwise, `1 + max(depth of each child)`.
    ///
    /// # Returns
    /// * `usize` - The depth of the tree.
    pub fn depth(&self, arena: &[Node]) -> usize {
        let mut max_depth = 0;
        let mut stack = vec![(self, 1)]; // (node, current_depth)

        while let Some((node, depth)) = stack.pop() {
            if depth > max_depth {
                max_depth = depth;
            }

            for &child_idx in &node.children {
                stack.push((&arena[child_idx], depth + 1));
            }
        }

        max_depth
    }

    /// Returns `true` if this node has no children.
    ///
    /// # Returns
    /// * `true` if the node is a leaf.
    /// * `false` otherwise.
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns a reference to this node's kind (`AstKind`).
    pub fn kind(&self) -> &AstKind {
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

    pub fn content(&self) -> &AstContent {
        &self.content
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

    /// Returns the identifier if this content is an `Ident`.
    ///
    /// # Returns
    ///
    /// - `Some(Ident)` if the content is an identifier.
    /// - `None` otherwise.
    pub fn as_ident(&self) -> Option<Ident> {
        self.content.as_ident()
    }

    /// Returns the floating-point literal if this content is a `Float`.
    ///
    /// # Returns
    ///
    /// - `Some(OrderedFloat<f64>)` if the content is a floating-point literal.
    /// - `None` otherwise.
    pub fn as_float(&self) -> Option<OrderedFloat<f64>> {
        self.content.as_float()
    }

    /// Returns the requirement flag if this content is a `Requirement`.
    ///
    /// # Returns
    ///
    /// - `Some(Requirement)` if the content is a requirement.
    /// - `None` otherwise.
    pub fn as_requirement(&self) -> Option<Requirement> {
        self.content.as_requirement()
    }

    /// Returns the binary comparison operator if this content is a `BinaryComp`.
    ///
    /// # Returns
    ///
    /// - `Some(BinaryComp)` if the content is a binary comparison operator.
    /// - `None` otherwise.
    pub fn as_binary_comp(&self) -> Option<BinaryComp> {
        self.content.as_binary_comp()
    }

    /// Returns the assignment operator if this content is an `AssignOp`.
    ///
    /// # Returns
    ///
    /// - `Some(AssignOp)` if the content is an assignment operator.
    /// - `None` otherwise.
    pub fn as_assign_op(&self) -> Option<AssignOp> {
        self.content.as_assign_op()
    }

    /// Returns the arithmetic operator if this content is an `ArithmeticOp`.
    ///
    /// # Returns
    ///
    /// - `Some(ArithmeticOp)` if the content is an arithmetic operator.
    /// - `None` otherwise.
    pub fn as_arithmetic_op(&self) -> Option<ArithmeticOp> {
        self.content.as_arithmetic_op()
    }

    /// Returns the optimization directive if this content is an `Optimization`.
    ///
    /// # Returns
    ///
    /// - `Some(Optimization)` if the content is an optimization directive.
    /// - `None` otherwise.
    pub fn as_optimization(&self) -> Option<Optimization> {
        self.content.as_optimization()
    }

    /// Returns `true` if the content is `None` (empty).
    ///
    /// # Returns
    ///
    /// - `true` if content is `AstContent::None`.
    /// - `false` otherwise.
    pub fn is_none(&self) -> bool {
        self.content.is_none()
    }

    /// Returns the identifier if this node's content is an `Ident`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not an `Ident`.
    pub fn try_ident(&self) -> Result<Ident, ParserInternalError> {
        self.content.try_ident()
    }

    /// Returns the floating-point literal if this node's content is a `Float`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not a `Float`.
    pub fn try_float(&self) -> Result<OrderedFloat<f64>, ParserInternalError> {
        self.content.try_float()
    }

    /// Returns the requirement if this node's content is a `Requirement`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not a `Requirement`.
    pub fn try_requirement(&self) -> Result<Requirement, ParserInternalError> {
        self.content.try_requirement()
    }

    /// Returns the binary comparison operator if this node's content is a `BinaryComp`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not a `BinaryComp`.
    pub fn try_binary_comp(&self) -> Result<BinaryComp, ParserInternalError> {
        self.content.try_binary_comp()
    }

    /// Returns the assignment operator if this node's content is an `AssignOp`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not an `AssignOp`.
    pub fn try_assign_op(&self) -> Result<AssignOp, ParserInternalError> {
        self.content.try_assign_op()
    }

    /// Returns the arithmetic operator if this node's content is an `ArithmeticOp`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not an `ArithmeticOp`.
    pub fn try_arithmetic_op(&self) -> Result<ArithmeticOp, ParserInternalError> {
        self.content.try_arithmetic_op()
    }

    /// Returns the optimization directive if this node's content is an `Optimization`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not an `Optimization`.
    pub fn try_optimization(&self) -> Result<Optimization, ParserInternalError> {
        self.content.try_optimization()
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
            "Node(kind={:?}, content={}, span={:?}, parent={:?}, children=[{}])",
            self.kind, self.content, self.span, self.parent, children_str
        )
    }
}
