//! Module `expr`
//!
//! This module defines the [`Expr`] struct, a wrapper around an abstract syntax tree (AST)
//! specialized to represent expressions in parsing and semantic analysis.
//!
//! The [`Expr`] struct encapsulates a generic [`Tree`] whose nodes are [`ExprNode`]s,
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

use crate::aiplan4rust::arena::iter::{PostorderIter, PreorderIter};
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{StringID, Optimization, RemapTypes, Type};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::expr::{normalize, ExprContent, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::{NodeId, Node, Tree};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::{Deref, DerefMut};
use crate::aiplan4rust::lir::renderers;
use crate::aiplan4rust::lir::renderers::{LiftedSyntaxDisplay, RenderContext};

/// Represents an expression tree, a wrapper around a [`Tree`] containing [`ExprNode`]s.
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
    tree: Tree<ExprNode>,
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
            tree: Tree::<ExprNode>::new(),
        }
    }

    pub fn kind(&self, id: NodeId) -> Result<ExprKind, ExprError> {
        Ok(self.try_node(id)?.kind())
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
    pub(crate) fn from_tree(tree: Tree<ExprNode>) -> Self {
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
        let root = ExprNode::new(ExprKind::Length, ExprContent::None, None);
        expr.alloc(root);
        expr
    }

    /// Returns the root node ID of the expression, if any.
    pub fn root_id(&self) -> Option<NodeId> {
        self.tree.root_id()
    }

    pub fn try_root_id(&self) -> Result<NodeId, SyntaxTreeError> {
        self.tree.try_root_id()
    }

    pub fn root_node(&self) -> Option<&ExprNode> {
        self.tree.root_node()
    }

    pub fn try_root_node(&self) -> Result<&ExprNode, SyntaxTreeError> {
        self.tree.try_node(self.try_root_id()?)
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
        self.tree.preorder()
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
        self.tree.postorder()
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
    pub fn deep_sub_expr_eq(&self, root_a: NodeId, root_b: NodeId) -> Result<bool, ExprError> {
        let iter1 = self.preorder_from(root_a).values();
        let iter2 = self.preorder_from(root_b).values();

        let mut i1 = iter1.peekable();
        let mut i2 = iter2.peekable();

        // Compare step-by-step
        for (n1, n2) in i1.by_ref().zip(i2.by_ref()) {
            if n1.kind() != n2.kind() {
                return Ok(false);
            }
            if n1.content() != n2.content() {
                return Ok(false);
            }
            if n1.children().len() != n2.children().len() {
                return Ok(false);
            }
        }

        // After the zip loop, check remaining nodes
        let leftover1 = i1.peek().is_some();
        let leftover2 = i2.peek().is_some();

        if leftover1 || leftover2 {
            return Ok(false); // different sizes => different structures
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
    /// let hash = expr.hash(node_id)?;
    /// ```
    #[allow(dead_code)]
    fn hash(&self, node_id: NodeId) -> Result<u64, ExprError> {
        let node = self.try_node(node_id)?;
        let mut hasher = DefaultHasher::new();

        // Hash the node type and content
        node.kind().hash(&mut hasher);
        node.content().hash(&mut hasher);

        // Recursively hash the children
        for &child in node.children() {
            let child_hash = self.hash(child)?;
            child_hash.hash(&mut hasher);
        }

        Ok(hasher.finish())
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
        Ok(self.set(node_id, ExprKind::And, Content::None, vec![])?)
    }

    /// Checks whether a node in the expression tree is an empty AND node `(and)`.
    ///
    /// An empty AND node has kind `ExprKind::And` and no children.
    /// In PDDL semantics, `(and)` represents a logical `true`.
    ///
    /// # Parameters
    /// - `node_id`: The ID of the node to check.
    ///
    /// # Returns
    /// - `Ok(true)` if the node exists, is of kind `And`, and has no children.
    /// - `Ok(false)` if the node exists but is not an empty AND.
    /// - `Err(ExprError)` if the node cannot be accessed.
    ///
    /// # Examples
    /// ```ignore
    /// let and_id = expr_builder.and(vec![]);
    /// assert!(expr.is_empty_and(and_id)?);
    /// ```
    pub fn is_empty_and(&self, node_id: NodeId) -> Result<bool, ExprError> {
        Ok(self.try_node(node_id)?.is_empty_and())
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
        Ok(self.set(node_id, ExprKind::Or, Content::None, vec![])?)
    }

    /// Checks whether a node in the expression tree is an empty OR node `(or)`.
    ///
    /// An empty OR node has kind `ExprKind::Or` and no children.
    /// In PDDL semantics, `(or)` represents a logical `false`.
    ///
    /// # Parameters
    /// - `node_id`: The ID of the node to check.
    ///
    /// # Returns
    /// - `Ok(true)` if the node exists, is of kind `Or`, and has no children.
    /// - `Ok(false)` if the node exists but is not an empty OR.
    /// - `Err(ExprError)` if the node cannot be accessed.
    ///
    /// # Examples
    /// ```ignore
    /// let or_id = expr_builder.or(vec![]);
    /// assert!(expr.is_empty_or(or_id)?);
    /// ```
    pub fn is_empty_or(&self, node_id: NodeId) -> Result<bool, ExprError> {
        Ok(self.try_node(node_id)?.is_empty_or())
    }

    /// Normalizes the expression in-place.
    ///
    /// This function applies the normalization process defined in the
    /// [`normalize`](crate::normalize) module to `self`. Normalization
    /// typically means transforming the expression into a standard or
    /// canonical form, which can be useful for comparison, evaluation,
    /// or optimization.
    ///
    /// # Errors
    ///
    /// Returns an [`ExprError`] if normalization fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use your_crate::{Expr, ExprError};
    /// # fn example() -> Result<(), ExprError> {
    /// let mut expr = Expr::new(...);
    /// expr.normalize()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// For more details on the normalization process and the rules applied,
    /// see the [`normalize`](crate::normalize) module.
    pub fn normalize(&mut self) -> Result<(), ExprError> {
        normalize(self)
    }
}

impl RemapTypes for Expr {
    /// Recursively remaps all union types (`Type::Either`) in the expression tree.
    ///
    /// # Parameters
    /// - `map`: A `HashMap` mapping each union type (`Type::Either`) to its corresponding primitive `Ident`.
    ///
    /// # Returns
    /// - `Ok(())` if all types were successfully remapped.
    /// - `Err(LirError)` if an error occurs during traversal or remapping.
    ///
    /// # Behavior
    /// - Traverses the expression tree from the root node.
    /// - Replaces type references according to `map`; non-union or unmapped types remain unchanged.
    fn remap_types(&mut self, map: &HashMap<Type<StringID>, StringID>) -> Result<(), LirError> {
        if let Ok(root_id) = self.tree.try_root_id() {
            self.flatten_types_from(root_id, map)?;
        }
        Ok(())
    }
}

impl Expr {
    /// Recursively remaps all union types (`Type::Either`) in this expression tree according to the provided mapping.
    ///
    /// This function traverses the expression tree starting from the given `node_id`.
    /// For each node:
    /// - If it contains quantified variables (`Forall` or `Exists`), their types are remapped.
    /// - All other node types are left unchanged.
    ///
    /// # Parameters
    /// - `node_id`: The `NodeId` of the root node to start traversal from.
    /// - `map`: A `HashMap<Type, Ident>` mapping union types (`Type::Either`) to their corresponding primitive `Ident`s.
    ///
    /// # Returns
    /// - `Ok(())` if all types were successfully remapped or are already primitive.
    /// - `Err(LirError)` if an error occurs during traversal or remapping (e.g., missing mapping or invalid node access).
    pub fn flatten_types_from(&mut self, node_id: NodeId, map: &HashMap<Type<StringID>, StringID>) -> Result<(), LirError> {
        let mut stack = vec![node_id];
        while let Some(node_id) = stack.pop() {
            let node = self.tree.try_node_mut(node_id)?;
            match node.kind() {
                ExprKind::Forall | ExprKind::Exists => {
                    //node.content_mut().try_quantifier_vars_mut()?.remap_types(map)?;
                }
                _ => {}
            }
            for &child_id in node.children() {
                stack.push(child_id);
            }
        }
        Ok(())
    }
}

impl Deref for Expr {
    type Target = Tree<ExprNode>;

    /// Dereferences `Expr` to the underlying [`Tree`] of [`ExprNode`]s.
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
    /// Mutable dereference to the underlying [`Tree`] of [`ExprNode`]s.
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
        renderers::default::render_expr(f, self)
    }
}

impl LiftedSyntaxDisplay for Expr {

    fn fmt_syntax(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> fmt::Result {
        renderers::syntax::expr::render(f, self, ctx)
    }
}
