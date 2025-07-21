use std::fmt;
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::arena::{ArenaNode, NodeId};
use crate::aiplan4rust::arena::iter::{PostorderIter, PostorderIterWithIndex, PreorderIdIter, PreorderIter, PreorderIterWithDepth, PreorderIterWithIndex};
use crate::aiplan4rust::arena::node_ref::{NodeRef, NodeRefMut};
use crate::aiplan4rust::arena::error::ArenaError;


/// A flat arena-based arena structure for storing nodes of type `T`.
///
/// The nodes are stored in a `Vec<T>`, and each syntax must implement the [`ArenaNode`] trait
/// which enables parent/child relationships through indices. This is useful for working
/// with abstract syntax trees and similar structures.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Arena<T: ArenaNode> {
    pub nodes: Vec<T>,
    root_id: Option<NodeId>,
}

impl<T: ArenaNode> Arena<T> {
    const DEFAULT_ROOT_ID: usize = 0;

    /// Creates a new, empty arena arena.
    pub fn new() -> Self {
        Arena {
            nodes: Vec::new(),
            root_id: Some(NodeId::new(Self::DEFAULT_ROOT_ID)),
        }
    }

    pub fn empty() -> Self {
        Arena {
            nodes: Vec::new(),
            root_id: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.root_id.is_none()
    }

    /// Adds a syntax into the arena and returns its `NodeId`.
    pub fn alloc(&mut self, node: T) -> NodeId {
        let id = NodeId::new(self.nodes.len());
        self.nodes.push(node);
        id
    }

    /// Returns a reference to the root syntax, if it exists.
    pub fn root_node(&self) -> Option<&T> {
        match self.root_id {
            Some(root_id) => self.get_node(root_id),
            None => None,
        }
    }

    /// Attempts to get an immutable reference to the root node.
    ///
    /// Returns an error if the root ID is missing or if the node ID is out of bounds.
    ///
    /// # Errors
    ///
    /// Returns `ArenaError::MissingRootId` if the root ID is not set.
    ///
    /// Returns other `ArenaError`s from `try_node`.
    pub fn try_root(&self) -> Result<&T, ArenaError> {
        match self.root_id {
            Some(id) => self.try_node(id),
            None => Err(ArenaError::missing_root_id()),
        }
    }

    /// Attempts to get a mutable reference to the root node.
    ///
    /// Returns an error if the root ID is missing or if the node ID is out of bounds.
    ///
    /// # Errors
    ///
    /// Returns `AiplanError::InternalError` if the root ID is missing.
    ///
    /// Returns `AiplanError::Arena` wrapping the specific `ArenaError` from `try_node_mut`.
    pub fn try_root_mut(&mut self) -> Result<&mut T, ArenaError> {
        match self.root_id {
            Some(id) => self.try_node_mut(id),
            None => Err(ArenaError::missing_root_id()),
        }
    }


    /// Returns a reference to the root syntax, if it exists.
    pub fn root_mut(&mut self) -> Option<&mut T> {
        match self.root_id {
            Some(root_id) => self.get_node_mut(root_id),
            None => None,
        }
    }

    /// Returns an immutable `NodeRef` to the root syntax, if it exists.
    pub fn root_node_ref(&self) -> Option<NodeRef<'_, T>> {
        match self.root_id {
            Some(root_id) => self.get_node_ref(root_id),
            None => None,
        }
    }

    pub fn root_id(&self) -> Option<NodeId> {
        self.root_id
    }

    /// Attempts to retrieve the root node ID.
    ///
    /// Returns an error if the root ID is missing.
    ///
    /// # Errors
    ///
    /// Returns `ArenaError::MissingRootId` if the root ID is not set.
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
    pub fn get_parent(&self, id: NodeId) -> Option<&T> {
        self.get_node(id)
            .and_then(|node| node.parent())
            .and_then(|parent_id| self.get_node(parent_id))
    }

    /// Returns an immutable reference to a syntax by its ID.
    pub fn get_node(&self, id: NodeId) -> Option<&T> {
        self.nodes.get(id.as_usize())
    }

    /// Returns a `NodeRef` combining the ID and an immutable reference.
    pub fn get_node_ref(&self, id: NodeId) -> Option<NodeRef<'_, T>> {
        self.nodes.get(id.as_usize()).map(|node| NodeRef::new(id, node))
    }

    /// Returns a mutable reference to a syntax by its ID.
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.nodes.get_mut(id.as_usize())
    }

    /// Returns a `NodeRefMut` combining the ID and a mutable reference.
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
        self.get_node(id).ok_or_else(|| ArenaError::node_not_found(index))
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

        self.get_node_mut(id).ok_or_else(|| ArenaError::node_not_found(index))
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
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns a preorder iterator starting from the root.
    pub fn preorder(&self) -> PreorderIter<'_, T> {
        match self.root_id {
            Some(root) => PreorderIter::new(self, root),
            None => PreorderIter::empty(self),
        }
    }

    /// Returns a preorder iterator over `NodeId`s starting from the root.
    ///
    /// This allows iteration where mutable or indexed access to the nodes is needed.
    ///
    /// # Example
    /// ```rust
    /// for node_id in arena.preorder_ids() {
    ///     let node = arena.get_node(node_id).unwrap();
    ///     // process syntax
    /// }
    /// ```
    pub fn preorder_ids(&self) -> PreorderIdIter<'_, T> {
        match self.root_id {
            Some(root) => PreorderIdIter::new(self, root),
            None => PreorderIdIter::empty(self),
        }
    }

    /// Returns a preorder iterator over `NodeId`s starting from the given syntax.
    pub fn preorder_ids_from(&self, root: NodeId) -> PreorderIdIter<'_, T> {
        PreorderIdIter::new(self, root)
    }

    /// Returns a preorder iterator from a specific syntax.
    pub fn preorder_from(&self, root: NodeId) -> PreorderIter<'_, T> {
        PreorderIter::new(self, root)
    }

    /// Returns a preorder iterator with indices from the root.
    pub fn preorder_with_index(&self) -> PreorderIterWithIndex<'_, T> {
        match self.root_id {
            Some(root) => PreorderIterWithIndex::new(self, root),
            None => PreorderIterWithIndex::empty(self),
        }
    }

    /// Returns a preorder iterator with depth from the root.
    pub fn preorder_with_depth(&self) -> PreorderIterWithDepth<'_, T> {
        match self.root_id {
            Some(root) => PreorderIterWithDepth::new(self, root),
            None => PreorderIterWithDepth::empty(self),
        }
    }

    /// Returns a postorder iterator from the root.
    pub fn postorder(&self) -> PostorderIter<'_, T> {
        match self.root_id {
            Some(root) => PostorderIter::new(self, root),
            None => PostorderIter::empty(self),
        }
    }

    /// Returns a postorder iterator from a specific syntax.
    pub fn postorder_from(&self, root: NodeId) -> PostorderIter<'_, T> {
        PostorderIter::new(self, root)
    }

    /// Returns a postorder iterator with indices from the root.
    pub fn postorder_with_index(&self) -> PostorderIterWithIndex<'_, T> {
        match self.root_id {
            Some(root) => PostorderIterWithIndex::new(self, root),
            None => PostorderIterWithIndex::empty(self),
        }
    }

    /// Computes the total number of nodes in a subtree.
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
}

impl<T> fmt::Display for Arena<T>
where
    T: ArenaNode + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn fmt_node<T: ArenaNode + fmt::Display>(
            arena: &Arena<T>,
            f: &mut fmt::Formatter<'_>,
            node: &T,
            node_index: usize,
            indent: usize,
            is_last: bool,
        ) -> fmt::Result {
            for _ in 0..indent {
                write!(f, "  ")?;
            }
            writeln!(f, "Node #{}: {}", node_index, node)?;

            let children = node.children();
            for child_idx in children.iter() {
                let child = arena.get_node(*child_idx).expect("Child not found");
                fmt_node(
                    arena,
                    f,
                    child,
                    child_idx.as_usize(),
                    indent + 1,
                    false,
                )?;
            }

            for _ in 0..indent {
                write!(f, "  ")?;
            }

            if is_last {
                write!(f, "End Node #{}", node_index)
            } else {
                writeln!(f, "End Node #{}", node_index)
            }
        }

        if self.is_empty() {
            write!(f, "<empty>")
        } else {
            let root = self.root_node().expect("root_node should exist if not empty");
            let root_index = self.root_id.expect("root_id should exist if not empty").as_usize();
            fmt_node(self, f, root, root_index, 0, true)
        }
    }
}
