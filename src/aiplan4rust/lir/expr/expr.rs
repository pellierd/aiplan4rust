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

use crate::aiplan4rust::arena::iter::{PostorderIter, PreorderIter};
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{FlattenTypes, Ident, Optimization, RemapIdents, Type};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::expr::{normalize, ExprContent, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxNode, SyntaxSubtree, SyntaxTree};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::{Deref, DerefMut};
use crate::aiplan4rust::lang::remap_idents::RemapIdentError;

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
        let root = ExprNode::new(ExprKind::Length, ExprContent::None, None);
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

    /// Recursively remaps all identifiers starting from a specific node in the tree.
    ///
    /// # Parameters
    /// - `id` – The root node of the subtree to apply remapping.
    /// - `map` – A `HashMap` mapping old `Ident`s to new `Ident`s.
    ///
    /// # Errors
    ///
    /// Returns a `RemapIdentError` if any identifier in the subtree fails to remap.
    pub fn remap_idents_from(&mut self, id: NodeId, map: &HashMap<Ident, Ident>) -> Result<(), RemapIdentError>{
        self.tree.remap_idents_from(id, map)?;
        Ok(())
    }

    /*/// Flattens types for the subtree starting at the given node.
    ///
    /// Traverses the subtree in a depth-first manner and replaces `Type` nodes
    /// with their flattened `Ident` from the provided mapping.
    /// Only nodes of kind `ExprKind::Type` are considered.
    ///
    /// # Parameters
    /// - `id`: The root `NodeId` of the subtree to flatten.
    /// - `map`: A mapping from `Type` to flattened `Ident`.
    pub fn flatten_types_from(&mut self, id: NodeId, map: &HashMap<Type, Ident>) {
        let mut stack = vec![id];

        while let Some(current_id) = stack.pop() {
            if let Some(node) = self.tree.get_node_mut(current_id) {
                // Only flatten nodes that are of kind Type
                if node.kind() == ExprKind::Type {
                    // Collect children's types
                    let mut members = Vec::new();
                    for &child_id in node.children() {
                        if let Some(child_node) = self.tree.get_node(child_id) {
                            if let Some(child_ident) = child_node.as_ident() {
                                members.push(child_ident);
                            }
                        }
                    }

                    // Build Type::Either from children
                    let ty = Type::either(members);

                    // Replace with flattened Ident if present in the map
                    if let Some(flattened_ident) = map.get(&ty) {
                        // Clear all existing children
                        node.children_mut().clear();

                        // Create a new primitive type node with the flattened Ident
                        let prim_node = ExprNode::new(ExprKind::PrimitiveType, ExprContent::Ident(*flattened_ident), None);
                        let prim_node_id = self.tree.alloc(prim_node);

                        // Assign it as the only child
                        node.add_child(prim_node_id);
                    }
                } else {
                    // Recurse into children
                    for &child_id in node.children() {
                        stack.push(child_id);
                    }
                }
            }
        }
    }*/

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

impl RemapIdents for Expr {
    /// Recursively remaps all identifiers in this expression.
    ///
    /// This updates every `Ident` contained within the expression's internal
    /// syntax tree according to the provided mapping. Useful when merging,
    /// flattening, or renaming symbols to ensure consistency across contexts.
    ///
    /// # Parameters
    ///
    /// - `map`: A `HashMap` mapping old `Ident` values to their new `Ident` values.
    ///
    /// # Behavior
    ///
    /// - The remapping is applied recursively to all nodes in the expression tree.
    /// - Identifiers not present in the mapping remain unchanged.
    ///
    /// # Errors
    ///
    /// Returns a `RemapIdentError` if remapping fails for any node in the tree.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) -> Result<(), RemapIdentError> {
        self.tree.remap_idents(map)?;
        Ok(())
    }
}

/*impl FlattenTypes for Expr {
    /// Recursively flattens all types in the expression tree.
    ///
    /// # Parameters
    /// - `map`: A mapping from `Type` to flattened `Ident` values.
    ///
    /// # Behavior
    /// - Traverses the expression tree starting from the root node.
    /// - Replaces type references in nodes according to the mapping.
    /// - Nodes whose type is not present in `map` remain unchanged.
    fn flatten_types(&mut self, map: &HashMap<Type, Ident>) {
        if let Ok(root_id) = self.tree.try_root_id() {
            self.flatten_types_from(root_id, map);
        }
    }
}*/

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
        let root_ast = subtree.node();

        // Create root node
        let root_kind = ExprKind::try_from(root_ast.kind())?;
        let root_content = ExprContent::try_from(root_ast.content())?;
        let root_node = ExprNode::new(root_kind, root_content, None);
        let root_id = expr.alloc(root_node);
        expr.set_root_id(root_id)?;

        let mut stack: Vec<(&AstNode, NodeId)> = Vec::new();

        // Helper function with explicit lifetime
        fn push_children<'a>(
            stack: &mut Vec<(&'a AstNode, NodeId)>,
            subtree: &'a SyntaxSubtree<'a, AstNode>,
            ast_node: &'a AstNode,
            parent_id: NodeId,
        ) -> Result<(), SyntaxTreeError> {
            for &child_id in ast_node.children().iter().rev() {
                let child_node = subtree.tree().try_node(child_id)?;
                stack.push((child_node, parent_id));
            }
            Ok(())
        }

        // Push root children
        push_children(&mut stack, subtree, root_ast, root_id)?;

        // DFS traversal
        while let Some((current_ast_node, parent_id)) = stack.pop() {
            let kind = ExprKind::try_from(current_ast_node.kind())?;
            let content = ExprContent::try_from(current_ast_node.content())?;

            let node = ExprNode::new(kind, content, Some(parent_id));
            let node_id = expr.alloc(node);

            expr.try_node_mut(parent_id)?.add_child(node_id);

            // Push children of the current node
            push_children(&mut stack, subtree, current_ast_node, node_id)?;
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
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.tree.fmt_with_interner(f, interner)
    }
}

impl SyntaxInternerDisplay for Expr {
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
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        self.tree
            .fmt_syntax_with_interner_and_indent(f, interner, indent)
    }
}
