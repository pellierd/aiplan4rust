//! Module providing an arena-based tree structure for managing hierarchical nodes.
//!
//! This module defines the `ArenaTree` struct, which stores nodes of type_checker `T` implementing
//! the [`ArenaNode`] trait in a flat vector, enabling efficient parent-child relationships
//! through indices.
//!
//! The arena supports allocation, retrieval, and mutation of nodes, along with traversal
//! via preorder and postorder iterators. It also offers utilities to query tree properties
//! such as size and depth.
//!
//! Errors related to invalid node access or missing roots are handled via [`ArenaError`].
//!
//! # Key types and traits
//! - `ArenaTree<T>`: Main tree structure managing nodes.
//! - `ArenaNode`: Trait required for nodes to track parent/children.
//! - `NodeId`: Unique identifier for nodes.
//! - `ArenaError`: Error type_checker for arena operations.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::aiplan4rust::compiler::syntax::ast::arena::error::ArenaError;
use crate::aiplan4rust::compiler::syntax::ast::arena::iter::{PostorderIter, PreorderIter};
use crate::aiplan4rust::compiler::syntax::ast::arena::node_ref::{NodeRef, NodeRefMut};
use crate::aiplan4rust::compiler::syntax::ast::arena::{ArenaNode, NodeId};

/// A flat arena-based tree structure for storing nodes of type_checker `T`.
///
/// The nodes are stored internally in a contiguous `Vec<T>`. Each node must implement the [`ArenaNode`] trait,
/// which enables parent-child relationships to be represented through indices rather than pointers.
/// This design is useful for representing hierarchical structures such as abstract syntax trees (ASTs),
/// parse trees, or any tree-like data where nodes can reference their children and parents by index.
///
/// # Type Parameters
///
/// - `T`: The node type_checker, which must implement the [`ArenaNode`] trait.
///
/// # Examples
///
/// ```
/// # use your_crate::{ArenaTree, ArenaNode, NodeId};
/// // Example node struct implementing ArenaNode would go here
/// #
/// # fn example() {
/// let mut arena = ArenaTree::<YourNodeType>::new();
/// let node_id = arena.alloc(YourNodeType::new());
/// // Work with arena and nodes...
/// # }
/// ```
///
/// # Notes
///
/// - The arena owns all nodes and manages them in a flat vector for cache efficiency.
/// - Nodes are identified and accessed by `NodeId`, which is simply an index wrapper.
/// - The tree keeps track of an optional root node ID, which can be `None` if empty.
///
/// # Serialization
///
/// This struct derives `Serialize` and `Deserialize` for easy persistence of the tree.
///
/// # Derives
///
/// `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `Serialize`, `Deserialize`, `Default`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ArenaTree<T: ArenaNode> {
    /// The internal storage of nodes in a contiguous vector.
    pub nodes: Vec<T>,

    /// Optional root node ID of the tree.
    root_id: Option<NodeId>,
}

impl<T: ArenaNode> ArenaTree<T> {
    /// Creates a new, empty arena.
    ///
    /// Note that this does not allocate any nodes initially.
    pub fn new() -> Self {
        ArenaTree {
            nodes: Vec::new(),
            root_id: None,
        }
    }

    /// Returns `true` if the arena contains no root node.
    ///
    /// This is a quick way to check if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.root_id.is_none()
    }

    /// Allocates a new node in the arena and returns its `NodeId`.
    ///
    /// The node is appended to the internal `Vec`, and its ID corresponds to its index.
    ///
    /// # Parameters
    ///
    /// - `node`: The node to insert into the arena.
    ///
    /// # Returns
    ///
    /// The `NodeId` corresponding to the inserted node.
    pub fn alloc(&mut self, node: T) -> NodeId {
        let id = NodeId::new(self.nodes.len());
        self.nodes.push(node);
        id
    }

    /// Allocates a new node in the arena and sets it as the root.
    ///
    /// This method appends the given node to the arena’s internal storage
    /// and updates the arena’s root ID to point to this node.
    /// If a root node already exists, it is replaced by the new node.
    ///
    /// # Parameters
    ///
    /// * `node` - The node to insert into the arena. This node will become
    ///   the root of the tree, replacing any existing root.
    ///
    /// # Returns
    ///
    /// * `NodeId` - The identifier corresponding to the newly allocated root node.
    pub fn alloc_root(&mut self, node: T) -> NodeId {
        let id = self.alloc(node);
        self.root_id = Some(id); // Remplace l'ancien root s'il existe
        id
    }

    /// Allocates a new node in the arena with the specified children.
    ///
    /// This method appends the given node to the arena’s internal storage,
    /// sets its children to the provided list, and updates the parent
    /// reference of each child to point to this node.
    ///
    /// # Parameters
    ///
    /// * `node` - The node to insert into the arena.
    /// * `children` - A vector of `NodeId` representing the children of the new node.
    ///
    /// # Returns
    ///
    /// * `NodeId` - The identifier corresponding to the newly allocated node.
    pub fn alloc_with_children(&mut self, mut node: T, children: Vec<NodeId>) -> NodeId {
        let id = self.alloc(node.clone());
        node.set_children(children.clone());
        for &child_id in &children {
            self.nodes[child_id.as_usize()].set_parent(Some(id));
        }
        self.nodes[id.as_usize()] = node;
        id
    }

    /// Allocates a new node in the arena with the specified children and sets it as the root.
    ///
    /// This method appends the given node to the arena’s internal storage,
    /// sets its children to the provided list, updates the parent reference
    /// of each child to point to this node, and sets this node as the root.
    /// If a root node already exists, it is replaced by the new node.
    ///
    /// # Parameters
    ///
    /// * `node` - The node to insert into the arena.
    /// * `children` - A vector of `NodeId` representing the children of the new root node.
    ///
    /// # Returns
    ///
    /// * `NodeId` - The identifier corresponding to the newly allocated root node.
    pub fn alloc_root_with_children(&mut self, node: T, children: Vec<NodeId>) -> NodeId {
        let id = self.alloc_with_children(node, children);
        self.root_id = Some(id);
        id
    }

    /// Returns a reference to the root node if it exists.
    ///
    /// Returns `None` if there is no root node set.
    pub fn root_node(&self) -> Option<&T> {
        match self.root_id {
            Some(root_id) => self.get_node(root_id),
            None => None,
        }
    }

    /// Attempts to get an immutable reference to the root node.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::MissingRootId`] if the root ID is not set.
    /// Returns other `ArenaError`s from `try_node` if the root ID is invalid.
    pub fn try_root(&self) -> Result<&T, ArenaError> {
        match self.root_id {
            Some(id) => self.try_node(id),
            None => Err(ArenaError::missing_root_id()),
        }
    }

    /// Attempts to get a mutable reference to the root node.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::MissingRootId`] if the root ID is not set.
    /// Returns other `ArenaError`s from `try_node_mut` if the root ID is invalid.
    pub fn try_root_mut(&mut self) -> Result<&mut T, ArenaError> {
        match self.root_id {
            Some(id) => self.try_node_mut(id),
            None => Err(ArenaError::missing_root_id()),
        }
    }

    /// Returns a mutable reference to the root node if it exists.
    ///
    /// Returns `None` if there is no root node.
    pub fn root_mut(&mut self) -> Option<&mut T> {
        match self.root_id {
            Some(root_id) => self.get_node_mut(root_id),
            None => None,
        }
    }

    /// Returns an immutable [`NodeRef`] to the root node if it exists.
    ///
    /// This can be useful for traversing or inspecting the node without copying.
    pub fn root_node_ref(&self) -> Option<NodeRef<'_, T>> {
        match self.root_id {
            Some(root_id) => self.get_node_ref(root_id),
            None => None,
        }
    }

    /// Attempts to return an immutable [`NodeRef`] to the root node.
    ///
    /// # Errors
    ///
    /// Returns an [`ArenaError`] if the root ID is not set or if the root node cannot be found.
    pub fn try_root_node_ref(&self) -> Result<NodeRef<'_, T>, ArenaError> {
        let root_id = self.try_root_id()?;
        self.get_node_ref(root_id)
            .ok_or_else(|| ArenaError::root_node_not_found(root_id.as_usize()))
    }

    /// Returns the root node ID if set.
    ///
    /// Returns `None` if the arena has no root node.
    pub fn root_id(&self) -> Option<NodeId> {
        self.root_id
    }

    /// Attempts to retrieve the root node ID.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::MissingRootId`] if the root ID is not set.
    pub fn try_root_id(&self) -> Result<NodeId, ArenaError> {
        match self.root_id {
            Some(id) => Ok(id),
            None => Err(ArenaError::missing_root_id()),
        }
    }

    /// Sets the root ID of the arena.
    ///
    /// # Arguments
    ///
    /// * `id` - The `NodeId` to set as the root.
    ///
    /// # Errors
    ///
    /// Returns an `ArenaError::NodeIdOutOfBounds` if the given `id` is outside the valid bounds
    /// of the arena nodes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::{Arena, NodeId, ArenaError};
    /// # let mut arena = Arena::new();
    /// # let node_id = NodeId::new(0);
    /// arena.set_root_id(node_id).expect("Valid root id");
    /// ```
    pub fn set_root_id(&mut self, id: NodeId) -> Result<(), ArenaError> {
        let idx = id.as_usize();
        let max = self.nodes.len().saturating_sub(1);

        if idx <= max {
            self.root_id = Some(id);
            Ok(())
        } else {
            Err(ArenaError::node_id_out_of_bounds(idx, max))
        }
    }

    /// Returns the parent syntax of a given syntax ID, if available.
    ///
    /// # Arguments
    ///
    /// * `id` - The `NodeId` whose parent is requested.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the parent node, or `None` if no parent exists or the node does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Some(parent) = arena.get_parent(node_id) {
    ///     println!("Parent found");
    /// }
    /// ```
    pub fn get_parent(&self, id: NodeId) -> Option<&T> {
        self.get_node(id)
            .and_then(|node| node.parent())
            .and_then(|parent_id| self.get_node(parent_id))
    }

    /// Returns an immutable reference to a syntax node by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The `NodeId` to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option` containing the reference to the node if it exists.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Some(node) = arena.get_node(node_id) {
    ///     // use node
    /// }
    /// ```
    pub fn get_node(&self, id: NodeId) -> Option<&T> {
        self.nodes.get(id.as_usize())
    }

    /// Returns a `NodeRef` combining the ID and an immutable reference.
    ///
    /// # Arguments
    ///
    /// * `id` - The `NodeId` to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option` containing a `NodeRef` if the node exists.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Some(node_ref) = arena.get_node_ref(node_id) {
    ///     // use node_ref
    /// }
    /// ```
    pub fn get_node_ref(&self, id: NodeId) -> Option<NodeRef<'_, T>> {
        self.nodes
            .get(id.as_usize())
            .map(|node| NodeRef::new(id, node))
    }

    /// Returns a mutable reference to a syntax node by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The `NodeId` to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option` containing a mutable reference if the node exists.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Some(node_mut) = arena.get_node_mut(node_id) {
    ///     // modify node_mut
    /// }
    /// ```
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.nodes.get_mut(id.as_usize())
    }

    /// Returns a `NodeRefMut` combining the ID and a mutable reference.
    ///
    /// # Arguments
    ///
    /// * `id` - The `NodeId` to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option` containing a `NodeRefMut` if the node exists.
    ///
    /// # Examples
    ///
    /// ```
    /// if let Some(node_ref_mut) = arena.get_ref_mut(node_id) {
    ///     // modify node_ref_mut
    /// }
    /// ```
    pub fn get_ref_mut(&mut self, id: NodeId) -> Option<NodeRefMut<'_, T>> {
        self.get_node_mut(id).map(|node| NodeRefMut::new(id, node))
    }

    /// Attempts to retrieve an immutable reference to a node by its `NodeId`.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the node to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(&T)` - A reference to the node if it exists and the id is within bounds.
    /// * `Err(ArenaError)` - An error if the `id` is out of bounds or the node is not found.
    ///
    /// # Errors
    ///
    /// Returns `ArenaError::NodeIdOutOfBounds` if the `id` exceeds the arena's node capacity.
    ///
    /// Returns `ArenaError::NodeNotFound` if the node at the given `id` does not exist.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let node = arena.try_node(node_id)?;
    /// println!("Node found: {:?}", node);
    /// # Ok::<(), ArenaError>(())
    /// ```
    pub fn try_node(&self, id: NodeId) -> Result<&T, ArenaError> {
        let index = id.as_usize();
        let max = self.nodes.len();
        if index >= max {
            return Err(ArenaError::node_id_out_of_bounds(index, max));
        }
        self.get_node(id)
            .ok_or_else(|| ArenaError::node_not_found(index))
    }

    /// Attempts to retrieve a `NodeRef` for the given `NodeId`.
    ///
    /// This method tries to get an immutable reference to the node identified by `id`,
    /// and if successful, wraps it in a `NodeRef`.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the node to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(NodeRef<'_, T>)` - A `NodeRef` wrapping the node if found.
    /// * `Err(ArenaError)` - An error if the node is not found or the id is out of bounds.
    ///
    /// # Errors
    ///
    /// Returns `ArenaError::NodeIdOutOfBounds` if `id` is outside the valid range of nodes.
    ///
    /// Returns `ArenaError::NodeNotFound` if the node at the specified `id` does not exist.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let node_ref = arena.try_node_ref(node_id)?;
    /// println!("NodeRef: {:?}", node_ref);
    /// # Ok::<(), ArenaError>(())
    /// ```
    pub fn try_node_ref(&self, id: NodeId) -> Result<NodeRef<'_, T>, ArenaError> {
        let node = self.try_node(id)?;
        Ok(NodeRef::new(id, node))
    }

    /// Attempts to retrieve a mutable reference to a node identified by `id`.
    ///
    /// This method checks if the given `NodeId` is within bounds and then tries to
    /// return a mutable reference to the node. If the node does not exist or the ID
    /// is out of bounds, an appropriate `ArenaError` is returned.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the node to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(&mut T)` - A mutable reference to the node if found.
    /// * `Err(ArenaError)` - An error indicating why the node could not be retrieved.
    ///
    /// # Errors
    ///
    /// Returns `ArenaError::NodeIdOutOfBounds` if `id` is greater than the maximum index.
    ///
    /// Returns `ArenaError::NodeNotFound` if the node at the specified `id` does not exist.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let node_mut = arena.try_node_mut(node_id)?;
    /// node_mut.update_something();
    /// # Ok::<(), ArenaError>(())
    /// ```
    pub fn try_node_mut(&mut self, id: NodeId) -> Result<&mut T, ArenaError> {
        let index = id.as_usize();
        let max = self.nodes.len().saturating_sub(1);

        if index > max {
            return Err(ArenaError::node_id_out_of_bounds(index, max));
        }

        self.get_node_mut(id)
            .ok_or_else(|| ArenaError::node_not_found(index))
    }

    /// Attempts to retrieve a mutable `NodeRefMut` for the node identified by `id`.
    ///
    /// This method attempts to get a mutable reference to the node via `try_node_mut`.
    /// If successful, it wraps the mutable reference in a `NodeRefMut` and returns it.
    /// If the node cannot be found or the ID is out of bounds, an appropriate `ArenaError` is returned.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the node to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(NodeRefMut<'_, T>)` - A mutable node reference wrapper if the node exists.
    /// * `Err(ArenaError)` - An error indicating why the node could not be retrieved.
    ///
    /// # Errors
    ///
    /// Returns `ArenaError::NodeIdOutOfBounds` if the `id` is out of bounds.
    ///
    /// Returns `ArenaError::NodeNotFound` if the node at the given `id` does not exist.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut node_ref = arena.try_node_ref_mut(node_id)?;
    /// node_ref.modify();
    /// # Ok::<(), ArenaError>(())
    /// ```
    pub fn try_node_ref_mut(&mut self, id: NodeId) -> Result<NodeRefMut<'_, T>, ArenaError> {
        let node = self.try_node_mut(id)?;
        Ok(NodeRefMut::new(id, node))
    }

    /// Returns the total number of nodes in the arena.
    ///
    /// # Returns
    ///
    /// The number of nodes.
    ///
    /// # Examples
    ///
    /// ```
    /// println!("Total nodes: {}", arena.len());
    /// ```
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns an iter that traverses the arena tree in preorder,
    /// starting from the root node if it exists.
    ///
    /// Preorder traversal visits the current node before its children,
    /// recursively from left to right.
    ///
    /// If the tree is empty (no root), returns an empty iter.
    ///
    /// # Examples
    ///
    /// ```
    /// for node in arena.preorder().values() {
    ///     // process each node in preorder
    /// }
    /// ```
    pub fn preorder(&self) -> PreorderIter<'_, T> {
        match self.root_id {
            Some(root) => PreorderIter::new(self, root),
            None => PreorderIter::empty(self),
        }
    }

    /// Returns a preorder iter starting from the given `root` node ID.
    ///
    /// This allows traversal of any subtree within the arena, starting
    /// at the specified node.
    ///
    /// Preorder traversal visits the current node before its children,
    /// recursively from left to right.
    ///
    /// # Arguments
    ///
    /// * `root` - The `NodeId` of the node where traversal should begin.
    ///
    /// # Examples
    ///
    /// ```
    /// let root = some_node_id;
    /// for node in arena.preorder_from(root).values() {
    ///     // process subtree in preorder
    /// }
    /// ```
    pub fn preorder_from(&self, root: NodeId) -> PreorderIter<'_, T> {
        PreorderIter::new(self, root)
    }

    /// Returns an iter that traverses the arena tree in postorder,
    /// starting from the root node if it exists.
    ///
    /// Postorder traversal visits the children of a node before the node itself,
    /// recursively from left to right.
    ///
    /// If the tree is empty (no root), returns an empty iter.
    ///
    /// # Examples
    ///
    /// ```
    /// for node in arena.postorder().values() {
    ///     // process each node in postorder
    /// }
    /// ```
    pub fn postorder(&self) -> PostorderIter<'_, T> {
        match self.root_id {
            Some(root) => PostorderIter::new(self, root),
            None => PostorderIter::empty(self),
        }
    }

    /// Returns a postorder iter starting from the given `root` node ID.
    ///
    /// This allows traversal of any subtree within the arena, starting
    /// at the specified node.
    ///
    /// Postorder traversal visits the children of a node before the node itself,
    /// recursively from left to right.
    ///
    /// # Arguments
    ///
    /// * `root` - The `NodeId` of the node where traversal should begin.
    ///
    /// # Examples
    ///
    /// ```
    /// let root = some_node_id;
    /// for node in arena.postorder_from(root).values() {
    ///     // process subtree in postorder
    /// }
    /// ```
    pub fn postorder_from(&self, root: NodeId) -> PostorderIter<'_, T> {
        PostorderIter::new(self, root)
    }

    /// Computes the total number of nodes in a subtree rooted at `root`.
    ///
    /// # Arguments
    ///
    /// * `root` - The root node of the subtree.
    ///
    /// # Returns
    ///
    /// The total number of nodes in the subtree.
    ///
    /// # Examples
    ///
    /// ```
    /// let size = arena.size(root_node_id);
    /// println!("Subtree size: {}", size);
    /// ```
    pub fn size(&self, root: NodeId) -> usize {
        let mut count = 0;
        let mut stack = vec![root];

        while let Some(node_id) = stack.pop() {
            count += 1;
            if let Some(node) = self.get_node(node_id) {
                for &child_id in node.children() {
                    stack.push(child_id);
                }
            }
        }

        count
    }

    /// Computes the depth of a subtree (max path from root to leaf).
    ///
    /// # Arguments
    ///
    /// * `root` - The root node of the subtree.
    ///
    /// # Returns
    ///
    /// The depth of the subtree.
    ///
    /// # Examples
    ///
    /// ```
    /// let depth = arena.depth(root_node_id);
    /// println!("Subtree depth: {}", depth);
    /// ```
    pub fn depth(&self, root: NodeId) -> usize {
        let mut max_depth = 0;
        let mut stack = vec![(root, 1)];

        while let Some((node_id, depth)) = stack.pop() {
            max_depth = max_depth.max(depth);
            if let Some(node) = self.get_node(node_id) {
                for &child_id in node.children() {
                    stack.push((child_id, depth + 1));
                }
            }
        }

        max_depth
    }

    /// Checks if the arena forms a valid tree structure starting from the root, detecting any cycles.
    ///
    /// A valid tree in this context must satisfy the following:
    /// - Each node is reachable from the root exactly once.
    /// - There are no cycles or re-entrant nodes (guaranteed by the tree definition).
    ///
    /// # Implementation Details
    ///
    /// This method performs a preorder traversal and uses an internal **BitSet**
    /// (packed `u64` vector) to track visited nodes. This is highly efficient:
    /// - **Memory**: Uses only 1 bit per node ($O(N/64)$ space).
    /// - **Performance**: Avoids the overhead of `HashSet` and provides $O(N)$
    ///   time complexity with excellent CPU cache locality.
    ///
    /// # Returns
    ///
    /// - `true` if the reachable part of the arena represents a valid tree.
    /// - `false` if a cycle is detected (a node is visited more than once).
    ///
    /// # Notes
    ///
    /// - An empty arena (no nodes) is considered a valid tree.
    /// - **Orphan nodes**: This check only validates the subgraph reachable from
    ///   the `root_id`. Nodes that are not connected to the root are ignored.
    ///
    /// # Example
    ///
    /// ```rust
    /// assert!(arena.is_tree());
    /// ```
    pub fn is_tree(&self) -> bool {
        let len = self.nodes.len();
        if len == 0 {
            return true;
        }

        let mut visited = vec![0u64; (len + 63) / 64];

        if let Some(root_id) = self.root_id {
            for (node_id, _) in self.preorder_from(root_id).ids() {
                let idx = node_id.as_usize();
                let word_idx = idx / 64;
                let bit_idx = idx % 64;
                let mask = 1 << bit_idx;

                if (visited[word_idx] & mask) != 0 {
                    return false;
                }
                visited[word_idx] |= mask;
            }
        }
        true
    }
}

/// Implements the [`Display`] trait for [`ArenaTree<T>`], where `T` implements both [`ArenaNode`] and [`Display`].
///
/// This allows you to pretty-print the arena tree using `format!` or `println!`.
/// Nodes are printed in a tree-like structure, starting from the root, with indentation
/// reflecting the depth in the tree. Each node is preceded by its index and followed
/// by an "End Node" marker.
///
/// If the arena is empty (i.e., has no root), it prints `"<empty>"`.
///
/// # Example
///
/// ```
/// # use crate::aiplan4rust::{ArenaTree, NodeId};
/// let mut arena = ArenaTree::new();
/// let node_id = arena.alloc(MyNode::new("root"));
/// arena.set_root_id(node_id).unwrap();
/// println!("{}", arena);
/// ```
///
/// # Errors
///
/// If any internal error occurs while traversing the tree (e.g., a child reference is invalid),
/// a [`fmt::Error`] is returned.
impl<T> fmt::Display for ArenaTree<T>
where
    T: ArenaNode + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        /// Recursively prints a node and its children with indentation.
        ///
        /// # Arguments
        ///
        /// * `arena` - The tree to traverse.
        /// * `f` - The formatter to write to.
        /// * `node` - The current node.
        /// * `node_index` - The index of the current node.
        /// * `indent` - The depth level in the tree.
        /// * `is_last` - Whether this is the last child of its parent.
        fn fmt_node<T: ArenaNode + fmt::Display>(
            arena: &ArenaTree<T>,
            f: &mut fmt::Formatter<'_>,
            node: &T,
            node_index: usize,
            indent: usize,
            _is_last: bool,
        ) -> fmt::Result {
            // Print indentation
            for _ in 0..indent {
                write!(f, "  ")?; // two spaces per indent level
            }

            // Print the node itself
            writeln!(f, "Node #{}: {}", node_index, node)?;

            // Recursively format all children
            for (i, child_id) in node.children().iter().enumerate() {
                let is_last_child = i == node.arity() - 1;
                if let Some(child_node) = arena.get_node(*child_id) {
                    fmt_node(
                        arena,
                        f,
                        child_node,
                        child_id.as_usize(),
                        indent + 1,
                        is_last_child,
                    )?;
                } else {
                    // If the child does not exist, print an error placeholder
                    for _ in 0..(indent + 1) {
                        write!(f, "  ")?;
                    }
                    writeln!(f, "Missing child #{}", child_id.as_usize())?;
                }
            }

            // Print the closing line for the current node
            for _ in 0..indent {
                write!(f, "  ")?;
            }

            writeln!(f, "End Node #{}", node_index)?;
            Ok(())
        }

        // If the tree has no root, it's considered empty
        if self.is_empty() {
            write!(f, "<empty>")
        } else {
            // Safely retrieve the root node and its index
            let root_id = self.root_id.ok_or(fmt::Error)?;
            let root_node = self.get_node(root_id).ok_or(fmt::Error)?;
            let root_index = root_id.as_usize();

            fmt_node(self, f, root_node, root_index, 0, true)
        }
    }
}
