//! Module `expr`
//!
//! This module defines the [`Expr`] struct, a wrapper around an abstract syntax tree (AST)
//! specialized to represent expressions in parsing and semantic analysis.
//!
//! The [`Expr`] struct encapsulates a generic [`SyntaxTree`] whose nodes are [`ExprNode`]s,
//! each associating an expression kind (`ExprKind`) with semantic content (`ExprContent`).
//!
//! This module also provides utility methods to create common predefined expressions,
//! such as empty expressions with logical `and` or `or` operators,
//! or expressions specific to metrics or length specifications.
//!
//! # Key Features
//! - Construction of empty expressions or with basic logical operators.
//! - Conversion from a generic AST subtree into a fully typed expression tree.
//! - Transparent access to the underlying tree via `Deref` and `DerefMut`.
//! - Displaying expressions with or without resolving interned identifiers via a `StringInterner`.
//!
//! # Examples
//!
//! ```rust
//! use aiplan4rust::lir::expr::Expr;
//!
//! let expr = Expr::new();
//! assert!(expr.is_empty());
//!
//! let expr_or = Expr::empty_or();
//! println!("{}", expr_or);
//! ```
//!
//! # Errors
//!
//! Converting from an AST subtree may fail if the conversion of kinds or content
//! is unsupported, returning an [`ExprError`].
//!

use std::collections::HashMap;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lir::expr::{ExprContent, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::SyntaxDisplay;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::{Deref, DerefMut};
use ahash::AHasher;
use crate::aiplan4rust::core::arena::iter::{PostorderIter, PreorderIter};
use crate::aiplan4rust::lang::{Ident, Optimization};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxSubtree, SyntaxTree};
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Represents an expression tree, a wrapper around a [`SyntaxTree`] containing [`ExprNode`]s.
///
/// This struct enables manipulation of expressions as syntax trees with precise semantic content,
/// facilitating construction, transformation, and display of expressions.
///
/// # Examples
///
/// ```rust
/// use aiplan4rust::lir::expr::Expr;
///
/// let expr = Expr::new();
/// assert!(expr.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Expr {
    tree: SyntaxTree<ExprNode>,
}

impl Expr {
    /// Creates a new, empty expression arena.
    ///
    /// # Examples
    ///
    /// ```
    /// let expr = Expr::new();
    /// assert!(expr.is_empty());
    /// ```
    ///
    /// # Returns
    ///
    /// A new instance of `Expr` with an empty underlying syntax tree.
    pub fn new() -> Self {
        Self {
            tree: SyntaxTree::<ExprNode>::new(),
        }
    }

    /// Constructs a new `Expr` from an existing `SyntaxTree<ExprNode>`.
    ///
    /// This function is intended for internal or crate-level usage only
    /// (`pub(crate)` visibility). It allows creating an `Expr` from a fully
    /// built syntax tree, for example after building it via an `ExprBuilder`.
    ///
    /// # Arguments
    ///
    /// * `tree` - The `SyntaxTree<ExprNode>` representing the expression structure.
    ///
    /// # Returns
    ///
    /// A new `Expr` instance that wraps the provided syntax tree.
    ///
    /// # Example
    ///
    /// ```
    /// use aiplan4rust::lir::expr::{Expr, ExprNode};
    /// use aiplan4rust::syntax::tree::SyntaxTree;
    ///
    /// let mut tree = SyntaxTree::<ExprNode>::new();
    /// // Build the tree...
    /// let expr = Expr::from_tree(tree);
    /// ```
    ///
    /// # Notes
    ///
    /// - This method bypasses any checks or invariants that might normally
    ///   be enforced by public constructors like `Expr::new()`.
    /// - It is designed to be used in conjunction with builders or internal
    ///   APIs where the tree is already guaranteed to be valid.
    pub(crate) fn from_tree(tree: SyntaxTree<ExprNode>) -> Self {
        Expr { tree }
    }

    /// Creates an expression with a single root node of kind `Or` and no content.
    ///
    /// This is useful to represent an empty logical OR expression.
    ///
    /// # Returns
    ///
    /// An `Expr` with root `ExprNode` of kind `Or` and empty content.
    pub fn empty_or() -> Self {
        let mut empty_or = Expr::new();
        let root = ExprNode::new(ExprKind::Or, ExprContent::None, None);
        empty_or.alloc(root);
        empty_or
    }

    /// Creates an expression with a single root node of kind `And` and no content.
    ///
    /// This is useful to represent an empty logical AND expression.
    ///
    /// # Returns
    ///
    /// An `Expr` with root `ExprNode` of kind `And` and empty content.
    pub fn empty_and() -> Self {
        let mut empty_and = Expr::new();
        let root = ExprNode::new(ExprKind::And, ExprContent::None, None);
        empty_and.alloc(root);
        empty_and
    }

    /// Creates an expression representing a metric with no optimization directive.
    ///
    /// This creates an expression with a root node of kind `Metric` and content specifying
    /// no optimization (`Optimization::None`).
    ///
    /// # Returns
    ///
    /// An `Expr` representing a metric expression with no optimization.
    pub fn metric_none() -> Self {
        let mut expr = Expr::new();
        let root = ExprNode::new(
            ExprKind::Metric,
            ExprContent::Optimization(Optimization::None),
            None,
        );
        expr.alloc(root);
        expr
    }

    /// Creates an expression with a single root node of kind `Length` and no content.
    ///
    /// This can represent an empty length specification expression.
    ///
    /// # Returns
    ///
    /// An `Expr` with root `ExprNode` of kind `Length` and empty content.
    pub fn empty_length_spec() -> Self {
        let mut expr = Expr::new();
        let root = ExprNode::new(
            ExprKind::Length,
            ExprContent::None,
            None,
        );
        expr.alloc(root);
        expr
    }

    /// Returns the root node ID of the expression, if any.
    pub fn root_id(&self) -> Option<NodeId> {
        self.tree.root_id()
    }

    /// Sets the root node ID of the expression.
    pub fn set_root_id(&mut self, id: NodeId) -> Result<(), SyntaxTreeError> {
        self.tree.set_root_id(id)
    }

    /// Allocates a new node in the expression tree.
    ///
    /// # Arguments
    /// * `node` - The expression node to be inserted.
    ///
    /// # Returns
    /// The unique [`NodeId`] assigned to the newly inserted node.
    pub fn alloc(&mut self, node: ExprNode) -> NodeId {
        self.tree.alloc(node)
    }

    /// Allocates a new node in the expression tree and sets it as the root.
    ///
    /// If a root already exists, it is replaced.
    ///
    /// # Arguments
    /// * `node` - The expression node to be inserted as the root.
    ///
    /// # Returns
    /// The unique [`NodeId`] assigned to the newly allocated root node.
    pub fn alloc_root(&mut self, node: ExprNode) -> NodeId {
        self.tree.alloc_root(node)
    }

    /// Allocates a new node in the expression tree with specified children.
    ///
    /// Updates the parent reference of each child to point to this node.
    ///
    /// # Arguments
    /// * `node` - The expression node to be inserted.
    /// * `children` - A vector of [`NodeId`] representing the children of the new node.
    ///
    /// # Returns
    /// The unique [`NodeId`] assigned to the newly inserted node.
    pub fn alloc_with_children(&mut self, node: ExprNode, children: Vec<NodeId>) -> NodeId {
        self.tree.alloc_with_children(node, children)
    }

    /// Allocates a new node in the expression tree with specified children and sets it as the root.
    ///
    /// If a root already exists, it is replaced.
    ///
    /// # Arguments
    /// * `node` - The expression node to be inserted as root.
    /// * `children` - A vector of [`NodeId`] representing the children of the new root node.
    ///
    /// # Returns
    /// The unique [`NodeId`] assigned to the newly allocated root node.
    pub fn alloc_root_with_children(&mut self, node: ExprNode, children: Vec<NodeId>) -> NodeId {
        self.tree.alloc_root_with_children(node, children)
    }


    /// Get an immutable reference to a node by ID.
    pub fn try_node(&self, id: NodeId) -> Result<&ExprNode, SyntaxTreeError> {
        self.tree.try_node(id)
    }

    /// Get a mutable reference to a node by ID.
    pub fn try_node_mut(&mut self, id: NodeId) -> Result<&mut ExprNode, SyntaxTreeError> {
        self.tree.try_node_mut(id)
    }

    /// Returns a preorder iterator starting at the root.
    ///
    /// # Returns
    /// A [`PreorderIter`] over all nodes from the root.
    pub fn preorder(&self) -> PreorderIter<ExprNode> {
        let root_id = self.root_id().expect("Expr has no root");
        self.tree.preorder_from(root_id)
    }

    /// Returns a preorder iterator starting at the specified node.
    ///
    /// # Arguments
    /// * `root` - The node ID to start traversal from.
    ///
    /// # Returns
    /// A [`PreorderIter`] beginning at `root`.
    pub fn preorder_from(&self, root: NodeId) -> PreorderIter<ExprNode> {
        self.tree.preorder_from(root)
    }

    /// Returns a postorder iterator starting at the root.
    ///
    /// # Returns
    /// A [`PostorderIter`] over all nodes from the root.
    pub fn postorder(&self) -> PostorderIter<ExprNode> {
        let root_id = self.root_id().expect("Expr has no root");
        self.tree.postorder_from(root_id)
    }

    /// Returns a postorder iterator starting at the specified node.
    ///
    /// # Arguments
    /// * `root` - The node ID to start traversal from.
    ///
    /// # Returns
    /// A [`PostorderIter`] beginning at `root`.
    pub fn postorder_from(&self, root: NodeId) -> PostorderIter<ExprNode> {
        self.tree.postorder_from(root)
    }

    /// Remaps identifiers across the whole expression tree.
    ///
    /// # Arguments
    /// * `map` - A mapping from old identifiers to new ones.
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        self.tree.remap_idents(map);
    }

    /// Remaps identifiers starting from a specific node in the tree.
    ///
    /// # Arguments
    /// * `id` - The root of the subtree to apply remapping.
    /// * `map` - A mapping of identifiers to apply.
    pub fn remap_idents_from(&mut self, id: NodeId, map: &HashMap<Ident, Ident>) {
        self.tree.remap_idents_from(id, map);
    }

    /// Computes a structural hash for a sub-expression (subtree) rooted at `node_id`.
    ///
    /// This hash can be used for deduplication, equality checking, or caching subtrees in PDDL-like ASTs.
    /// The hash is **structural**, meaning it considers the node type, the node content, and all children recursively.
    /// For commutative nodes (AND and OR), child order does not affect the hash, ensuring that (and A B) and (and B A) produce the same hash.
    ///
    /// # Parameters
    /// - `node_id`: The ID of the node to hash. This node serves as the root of the sub-expression.
    /// - `self`: A reference to the expression tree containing the node and its children.
    ///
    /// # Returns
    /// - `Ok(u64)`: A 64-bit hash representing the structure of the sub-expression.
    /// - `Err(ExprError)`: If the `node_id` is invalid or cannot be accessed.
    ///
    /// # Behavior
    /// 1. Retrieves the node corresponding to `node_id`.
    /// 2. Hashes the kind of the node (ExprKind) to distinguish types (Predicate, And, Or, Not, etc.).
    /// 3. Hashes the node content (ExprContent), ensuring that nodes of the same kind but different content produce different hashes.
    /// 4. Recursively computes hashes for all children.
    ///    - For commutative nodes (AND, OR), child hashes are sorted before combining, making the hash insensitive to child order.
    ///    - For non-commutative nodes, child hashes are combined in the original order.
    /// 5. Combines all child hashes into the node's hasher to produce a final hash.
    ///
    /// # Notes
    /// - The function is recursive and works for any depth of the expression tree.
    /// - It requires that `ExprContent` implements `Hash`.
    /// - It produces a consistent hash for structurally equivalent subtrees, suitable for structural deduplication.
    /// - Commutative nodes (AND/OR) are insensitive to the order of their children.
    ///
    /// # Example
    /// ```ignore
    /// // Suppose expr contains: (and A (or B C)) or (and (or C B) A)
    /// let root_id = expr.root_id().unwrap();
    /// let hash = expr.sub_expr_hash(root_id)?;
    /// println!("Hash of root subtree: {}", hash);
    /// ```
    pub fn sub_expr_hash(&self, node_id: NodeId) -> Result<u64, ExprError> {
        let node = self.try_node(node_id)?;
        let mut hasher = AHasher::default();

        // Hash the node type first
        node.kind().hash(&mut hasher);

        // Hash the node content (name, value, arguments, etc.)
        // This ensures nodes of the same kind but different content have distinct hashes
        node.content().hash(&mut hasher);

        // Recursively hash all children
        let mut child_hashes = Vec::with_capacity(node.children().len());
        for &child_id in node.children() {
            let h = self.sub_expr_hash(child_id)?;
            child_hashes.push(h);
        }

        // For commutative nodes, sort child hashes so order does not matter
        match node.kind() {
            ExprKind::And | ExprKind::Or => child_hashes.sort_unstable(),
            _ => {}
        }

        // Combine all child hashes into the node hash
        for h in child_hashes {
            h.hash(&mut hasher);
        }

        Ok(hasher.finish())
    }

    /// Compare two subtrees of possibly different `Expr`s for deep equality.
    ///
    /// # Parameters
    /// - `other`: The other expression to compare with.
    /// - `a`: NodeId of the root of the subtree in `self`.
    /// - `b`: NodeId of the root of the subtree in `other`.
    ///
    /// # Returns
    /// - `Ok(true)` if the subtrees are structurally and content-wise equal.
    /// - `Ok(false)` otherwise.
    ///
    /// # Notes
    /// - Children of the nodes **must be sorted** if you want this comparison to be
    ///   independent of the order of children. Otherwise, the comparison is order-sensitive.
    /// - This function compares recursively: node kind, content, and all children.
    /// Compare two subtrees rooted at `a` in `self` and `b` in `other` for deep equality.
    /// Children should already be sorted if order does not matter.
    pub fn deep_sub_expr_eq(
        &self,
        root_a: NodeId,
        root_b: NodeId,
    ) -> Result<bool, ExprError> {
        let iter1 = self.preorder_from(root_a).values();
        let iter2 = self.preorder_from(root_b).values();

        // Iterate over both trees in preorder
        for (n1, n2) in iter1.zip(iter2) {
            // Compare node kind
            if n1.kind() != n2.kind() {
                return Ok(false);
            }

            // Compare node content
            if n1.content() != n2.content() {
                return Ok(false);
            }

            // Compare number of children
            if n1.children().len() != n2.children().len() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Computes the hash of a subtree of the expression.
    ///
    /// The hash combines:
    /// - the node's type (`kind`),
    /// - the node's content (`content`),
    /// - the recursive hash of each child.
    ///
    /// This version **does not use caching**, so the hash is recalculated on each call.
    /// Children should be sorted beforehand if order should not affect the result.
    ///
    /// # Parameters
    /// - `node_id`: the ID of the root node of the subtree to hash.
    ///
    /// # Returns
    /// - `Ok(u64)` containing the computed hash of the subtree.
    /// - `Err(ExprError)` if accessing a node fails.
    ///
    /// # Example
    /// ```ignore
    /// let hash = expr.compute_hash(node_id)?;
    /// ```
    fn compute_hash(&self, node_id: NodeId) -> Result<u64, ExprError> {
        let node = self.try_node(node_id)?;
        let mut hasher = DefaultHasher::new();

        // Hash the node type and content
        node.kind().hash(&mut hasher);
        node.content().hash(&mut hasher);

        // Recursively hash the children
        for &child in node.children() {
            let child_hash = self.compute_hash(child)?;
            child_hash.hash(&mut hasher);
        }

        Ok(hasher.finish())
    }

    /// Sets the kind, content, and children of a node at a given position.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to update.
    /// * `kind` - The new kind to assign to the node.
    /// * `content` - The new content to assign to the node.
    /// * `children` - The new children of the node.
    ///
    /// # Notes
    ///
    /// This function **replaces** all aspects of the node in a single operation.
    /// Any previous content or children are overwritten.
    ///
    /// # Example
    ///
    /// ```rust
    /// let children = vec![child1_id, child2_id];
    /// expr.set_node(node_id, ExprKind::And, Content::None, children)?;
    /// ```
    pub fn set(
        &mut self,
        node_id: NodeId,
        kind: ExprKind,
        content: Content,
        children: Vec<NodeId>,
    ) -> Result<(), ExprError> {
        let node = self.try_node_mut(node_id)?;
        node.set_kind(kind);
        node.set_content(content);
        node.set_children(children);
        Ok(())
    }

    /// Moves the kind, content, and children from a source node into a target node.
    ///
    /// # Arguments
    ///
    /// * `source_id` - The ID of the node to move data from.
    /// * `target_id` - The ID of the node to move data to.
    ///
    /// # Notes
    ///
    /// This function **takes ownership** of the source node's content and children,
    /// leaving the source node effectively empty. This is useful for in-place
    /// simplifications or transformations without cloning nodes.
    ///
    /// # Example
    ///
    /// ```rust
    /// expr.move_to(source_id, target_id)?;
    /// ```
    pub fn move_to(&mut self, source_id: NodeId, target_id: NodeId) -> Result<(), ExprError> {
        let (kind, content, children) = {
            let source = self.try_node_mut(source_id)?;
            (
                source.kind(),
                std::mem::take(source.content_mut()),
                std::mem::take(source.children_mut()),
            )
        };
        self.set(target_id, kind, content, children)?;
        Ok(())
    }

    /// Sets the node at `node_id` to an empty `(and)` node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to modify.
    ///
    /// # Notes
    ///
    /// This sets the node kind to `ExprKind::And`, clears its content, and
    /// removes all children.
    ///
    /// # Example
    ///
    /// ```rust
    /// expr.set_empty_and(node_id)?;
    /// ```
    pub fn set_empty_and(&mut self, node_id: NodeId) -> Result<(), ExprError> {
        self.set(node_id, ExprKind::And, Content::None, vec![])
    }

    /// Sets the node at `node_id` to an empty `(or)` node.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The ID of the node to modify.
    ///
    /// # Notes
    ///
    /// This sets the node kind to `ExprKind::Or`, clears its content, and
    /// removes all children.
    ///
    /// # Example
    ///
    /// ```rust
    /// expr.set_empty_or(node_id)?;
    /// ```
    pub fn set_empty_or(&mut self, node_id: NodeId) -> Result<(), ExprError> {
        self.set(node_id, ExprKind::Or, Content::None, vec![])
    }
}

/// Attempts to build an [`Expr`] from a given [`SyntaxSubtree`] referencing an AST node and its syntax tree.
///
/// This implementation iteratively traverses the AST subtree, converting each node to an `ExprNode`
/// while preserving parent-child relationships to construct the expression tree.
///
/// # Errors
///
/// Returns an [`ExprError`] if:
/// - Conversion of AST kinds or contents to expression kinds or contents fails.
/// - Syntax node lookups fail.
///
/// # Arguments
///
/// * `subtree` - A reference to a [`SyntaxSubtree`] of [`AstNode`] representing the AST fragment.
///
/// # Returns
///
/// Returns an `Expr` instance constructed from the AST subtree on success.
///
/// # Example
///
/// ```rust,ignore
/// let subtree: &SyntaxSubtree<AstNode> = ...;
/// let expr = Expr::try_from(subtree)?;
/// ```
impl TryFrom<&SyntaxSubtree<'_, AstNode>> for Expr {
    type Error = ExprError;

    fn try_from(subtree: &SyntaxSubtree<'_, AstNode>) -> Result<Self, Self::Error> {
        let mut expr = Expr::new();
        let mut stack = Vec::new();

        // Stack elements: (AST node, optional parent ExprNodeId)
        stack.push((subtree.node(), None));

        while let Some((current_ast_node, parent_expr_id_opt)) = stack.pop() {
            // Convert AST kind and content to Expr kind and content
            let kind = ExprKind::try_from(current_ast_node.kind())?;
            let content = ExprContent::try_from(current_ast_node.content())?;

            // Create ExprNode and allocate in the expression arena
            let expr_node = ExprNode::new(kind, content, parent_expr_id_opt);
            let expr_node_id = expr.alloc(expr_node);

            // Link this node as child of parent if parent exists
            if let Some(parent_id) = parent_expr_id_opt {
                expr.try_node_mut(parent_id)?.add_child(expr_node_id);
            }

            // Add children of the current AST node to the stack in reverse order,
            // to maintain left-to-right traversal order
            for &child_id in current_ast_node.children().iter().rev() {
                let child_node = subtree.tree().try_node(child_id)?;
                stack.push((child_node, Some(expr_node_id)));
            }
        }

        Ok(expr)
    }
}

impl Deref for Expr {
    type Target = SyntaxTree<ExprNode>;

    /// Dereferences `Expr` to the underlying [`SyntaxTree`] of [`ExprNode`]s.
    ///
    /// This enables convenient transparent access to all tree operations on the expression.
    ///
    /// # Returns
    ///
    /// A reference to the internal syntax tree.
    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl DerefMut for Expr {
    /// Mutable dereference to the underlying [`SyntaxTree`] of [`ExprNode`]s.
    ///
    /// Allows mutation of the expression tree structure.
    ///
    /// # Returns
    ///
    /// A mutable reference to the internal syntax tree.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl fmt::Display for Expr {
    /// Formats the expression as a string using the Display implementation of the underlying syntax tree.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    ///
    /// # Returns
    ///
    /// A formatting result indicating success or failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.tree, f)
    }
}

impl InternerDisplay for Expr {
    /// Formats the expression using a [`StringInterner`] to resolve interned identifiers.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    /// * `interner` - The interner used to resolve interned strings.
    ///
    /// # Returns
    ///
    /// A formatting result indicating success or failure.
    fn fmt_with_interner(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.tree.fmt_with_interner(f, interner)
    }
}

impl SyntaxDisplay for Expr {
    /// Formats the expression with indentation and interner support for pretty printing.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    /// * `interner` - The string interner for resolving identifiers.
    /// * `indent` - The indentation level (number of spaces or tabs).
    ///
    /// # Returns
    ///
    /// A formatting result indicating success or failure.
    fn fmt_syntax_with_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        self.tree.fmt_syntax_with_indent(f, interner, indent)
    }
}
