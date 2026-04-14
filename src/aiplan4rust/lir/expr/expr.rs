//! Module `logic`
//!
//! This module defines the [`Expr`] struct, a wrapper around an abstract syntax tree (AST)
//! specialized to represent logic in parsing and semantic analysis.
//!
//! The [`Expr`] struct encapsulates a generic [`Tree`] whose nodes are [`ExprNode`]s,
//! each associating an expression kind (`ExprKind`) with semantic content (`ExprContent`).
//!
//! This module also provides utility methods to create common predefined logic,
//! such as empty logic with logical `and` or `or` operators,
//! or logic specific to metrics or length specifications.
//!
//! # Key Features
//! - Construction of empty logic or with basic logical operators.
//! - Conversion from a generic AST subtree into a fully typed expression tree.
//! - Transparent access to the underlying tree via `Deref` and `DerefMut`.
//! - Displaying logic with or without resolving interned identifiers via a `StringInterner`.
//!
//! # Examples
//!
//! ```rust
//! use aiplan4rust::lir::logic::Expr;
//!
//! let logic = Expr::new();
//! assert!(logic.is_empty());
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
use crate::aiplan4rust::lang::{ObjectId, OptimizationOp};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::expr::{ExprContent, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::lir::renderers;
use crate::aiplan4rust::lir::renderers::{LiftedSyntaxDisplay, RenderContext};
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::aiplan4rust::tree::{Node, NodeId, Tree};
use core::fmt::Formatter;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};

/// Represents an expression tree, a wrapper around a [`Tree`] containing [`ExprNode`]s.
///
/// This struct enables manipulation of logic as syntax trees with precise semantic content,
/// facilitating construction, transformation, and display of logic.
///
/// # Examples
///
/// ```rust
/// use aiplan4rust::lir::logic::Expr;
///
/// let logic = Expr::new();
/// assert!(logic.is_empty());
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

    /// Returns `true` if the expression contains no nodes.
    ///
    /// An empty expression is defined as having no root node.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.root_id().is_none()
    }

    pub fn hash(&self, id: NodeId) -> u64 {
        let node = self.try_node(id).expect("Le nœud doit exister");

        // 1. On vérifie le cache du nœud
        if let Some(h) = node.hash() {
            return h;
        }

        // 2. Si None (sale), on calcule récursivement
        let mut hasher = fxhash::FxHasher::default();
        node.kind().hash(&mut hasher);
        node.content().hash(&mut hasher);

        for &child_id in node.children() {
            // L'appel récursif profitera des caches des enfants non modifiés
            let child_h = self.hash(child_id);
            hasher.write_u64(child_h);
        }

        let final_h = hasher.finish();

        // 3. On remplit le cache du nœud (grâce au Cell)
        node.set_hash(final_h);
        final_h
    }

    pub fn invalidate(&self, id: NodeId) {
        let mut current = Some(id);
        while let Some(curr_id) = current {
            let node = self.try_node(curr_id).unwrap();

            // Si c'est déjà None, on arrête : les ancêtres sont déjà invalidés.
            if node.hash().is_none() {
                break;
            }

            node.invalidate();
            current = node.parent(); // Remonte vers le parent
        }
    }

    pub fn set_to_bool(&mut self, node_id: NodeId, value: bool) -> Result<bool, ExprError> {
        let node_mut = self.try_node_mut(node_id)?;
        let kind = if value { ExprKind::And } else { ExprKind::Or };
        node_mut.set_kind(kind);
        node_mut.children_mut().clear();
        Ok(true)
    }

    /// Remplace un nœud existant par une constante numérique (Number).
    /// Utile pour le constant folding et la réduction d'inertie.
    pub fn set_to_number(
        &mut self,
        node_id: NodeId,
        val: OrderedFloat<f64>,
    ) -> Result<(), ExprError> {
        let node_mut = self.try_node_mut(node_id)?;
        node_mut.set_kind(ExprKind::Number);
        node_mut.set_content(Content::Number(val));
        node_mut.children_mut().clear();
        Ok(())
    }

    /// Version pour les objets (Constant)
    pub fn set_to_object(&mut self, node_id: NodeId, obj_id: ObjectId) -> Result<(), ExprError> {
        let node_mut = self.try_node_mut(node_id)?;
        node_mut.set_kind(ExprKind::Object);
        node_mut.set_content(Content::Object(obj_id));
        node_mut.children_mut().clear();
        Ok(())
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
            ExprContent::OptimizationOp(OptimizationOp::None),
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

    pub fn get_node_kind(&self, id: NodeId) -> Option<ExprKind> {
        self.get_node(id).map(|node| node.kind())
    }

    pub fn try_node_kind(&self, id: NodeId) -> Result<ExprKind, ExprError> {
        Ok(self.try_node(id)?.kind())
    }

    /// Returns the kind of the root node of the expression, if any.
    pub fn kind(&self) -> Option<ExprKind> {
        self.root_id().and_then(|id| self.get_node_kind(id))
    }

    /// Tries to return the kind of the root node, or an error if the root is not set.
    pub fn try_kind(&self) -> Result<ExprKind, ExprError> {
        self.try_node_kind(self.try_root_id()?)
    }

    /// Allocates a new node in the expression tree.
    ///
    /// # Arguments
    /// * `node` - The expression node to be inserted.
    ///
    /// # Returns
    /// The unique [`NodeId`] assigned to the newly inserted node.
    pub fn alloc(&mut self, node: ExprNode) -> NodeId {
        // Si le nœud qu'on alloue a déjà un ID de parent défini
        let parent_to_invalidate = node.parent();

        let new_id = self.tree.alloc(node);

        // Si on l'insère alors qu'il est déjà lié à un parent,
        // il faut dire au parent que sa structure a changé.
        if let Some(pid) = parent_to_invalidate {
            self.invalidate(pid);
        }

        new_id
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
        self.invalidate(id);
        self.tree.try_node_mut(id)
    }

    /// Returns a preorder iter starting at the root.
    ///
    /// # Returns
    /// A [`PreorderIter`] over all nodes from the root.
    pub fn preorder(&self) -> PreorderIter<ExprNode> {
        self.tree.preorder()
    }

    /// Returns a preorder iter starting at the specified node.
    ///
    /// # Arguments
    /// * `root` - The node ID to start traversal from.
    ///
    /// # Returns
    /// A [`PreorderIter`] beginning at `root`.
    pub fn preorder_from(&self, root: NodeId) -> PreorderIter<ExprNode> {
        self.tree.preorder_from(root)
    }

    /// Returns a postorder iter starting at the root.
    ///
    /// # Returns
    /// A [`PostorderIter`] over all nodes from the root.
    pub fn postorder(&self) -> PostorderIter<ExprNode> {
        self.tree.postorder()
    }

    /// Returns a postorder iter starting at the specified node.
    ///
    /// # Arguments
    /// * `root` - The node ID to start traversal from.
    ///
    /// # Returns
    /// A [`PostorderIter`] beginning at `root`.
    pub fn postorder_from(&self, root: NodeId) -> PostorderIter<ExprNode> {
        self.tree.postorder_from(root)
    }

    /// Checks for deep structural equality between two sub-expressions.
    ///
    /// This method determines if two expression trees are semantically identical,
    /// even if they are composed of nodes with different `NodeId`s (distinct physical identity).
    ///
    /// ### Algorithm and Performance
    /// The implementation follows a "Hash-based structural equality" approach:
    /// 1. **Physical Identity (O(1))**: If IDs are identical, the trees are guaranteed to be the same.
    /// 2. **Hash Short-circuit (Amortized O(1))**: If hashes differ, the trees are guaranteed
    ///    to be different. Thanks to hash caching, this step prevents unnecessary recursion.
    /// 3. **Safety Validation & Recursion**: In case of identical hashes (potential collision
    ///    or structural matching), it verifies node content and recursively traverses children.
    ///
    /// ### Use Case:
    /// Essential for simplifying trivial equalities like `(= (f ?x) (f ?x))` where both
    /// calls to `f` might originate from different branches of the AST and thus have different IDs.
    ///
    /// # Arguments
    /// * `root_a` - Root ID of the first sub-expression.
    /// * `root_b` - Root ID of the second sub-expression.
    ///
    /// # Returns
    /// * `Ok(true)` if expressions are structurally identical.
    /// * `Err(ExprError)` if a NodeId is invalid during traversal.
    pub fn deep_sub_expr_eq(&self, root_a: NodeId, root_b: NodeId) -> Result<bool, ExprError> {
        // 1. Physical identity: if it's the same ID, it's the same node (O(1))
        if root_a == root_b {
            return Ok(true);
        }

        // 2. Hash filter: if hashes differ, they are definitely different.
        // This is the key step that makes deep comparison highly efficient.
        if self.hash(root_a) != self.hash(root_b) {
            return Ok(false);
        }

        // 3. Structural & Anti-collision safety check
        let n1 = self.try_node(root_a)?;
        let n2 = self.try_node(root_b)?;

        if n1.kind() != n2.kind() || n1.content() != n2.content() {
            return Ok(false);
        }

        // 4. DEEP RECURSION
        // We do not compare children IDs directly (==) because they could be
        // structurally identical but physically distinct.
        let c1 = n1.children();
        let c2 = n2.children();

        if c1.len() != c2.len() {
            return Ok(false);
        }

        // Use deep_sub_expr_eq recursively instead of checking IDs equality
        for (&child_a, &child_b) in c1.iter().zip(c2.iter()) {
            if !self.deep_sub_expr_eq(child_a, child_b)? {
                return Ok(false);
            }
        }

        Ok(true)
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
        self.invalidate(node_id);
        self.set(node_id, ExprKind::And, Content::None, vec![])?;
        Ok(())
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
    /// assert!(logic.is_empty_and(and_id)?);
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
        self.invalidate(node_id);
        self.set(node_id, ExprKind::Or, Content::None, vec![])?;
        Ok(())
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
    /// assert!(logic.is_empty_or(or_id)?);
    /// ```
    pub fn is_empty_or(&self, node_id: NodeId) -> Result<bool, ExprError> {
        Ok(self.try_node(node_id)?.is_empty_or())
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
