//! Trait defining the interface for syntax nodes stored in an arena-based tree structure.
//!
//! This module provides the common abstraction [`ArenaNode`] for representing syntax
//! nodes managed by an arena (`TreeArena`). It specifies essential methods for
//! navigating parent-child relationships, modifying node structure, and accessing
//! semantic content within the arena.
//!
//! # Overview
//!
//! The `ArenaNode` trait establishes a contract for node types to be compatible
//! with arena storage and traversal. Nodes implementing this trait can:
//! - Access and mutate parent and children node references by `NodeId`.
//! - Add and replace child nodes safely.
//! - Retrieve parent or child nodes with error handling for missing or invalid references.
//! - Support semantic operations like remapping identifiers and extracting symbols
//!   via associated content types.
//!
//! # Design Notes
//!
//! - Methods use `Result` to avoid panics and provide detailed error information
//!   with [`ArenaError`] variants.
//! - The trait requires implementors to be `Clone` and `Debug` to facilitate
//!   copying and debugging.
//! - Intended for use with arena-managed trees where nodes are stored contiguously,
//!   identified by unique `NodeId`s.
//!
//! # Related Types
//!
//! - [`ArenaTree`] — The arena manager handling nodes implementing `ArenaNode`.
//! - [`NodeContent`] — The semantic content trait associated with syntax nodes.
//! - [`ArenaError`] — Error enum for robust error handling in tree operations.
//! - [`SymbolRef`] — For representing symbols extracted from syntax nodes.
//!
//! # Example
//!
//! See [`ArenaNode`] for usage examples on safe traversal and mutation of arena nodes.

use crate::aiplan4rust::syntax::ast::arena::{ArenaError, NodeId};
use std::fmt::Debug;

/// A generic trait representing syntax stored within an arena (`TreeArena`).
///
/// This trait defines the minimal interface any syntax node type_checker must implement
/// to be used in an arena-managed tree structure. It includes methods for
/// navigating parent-child relationships, modifying the arena, accessing content,
/// and extracting semantic information like identifiers and symbols.
///
/// # Core Responsibilities
///
/// - Access and modify the node’s kind and content.
/// - Navigate the arena: get parent and children, add children.
/// - Remap identifiers within the content using a mapping.
/// - Query properties like leaf/root status and number of children.
/// - Attempt to extract symbol references from the syntax.
///
/// # Notes
///
/// - The trait expects an associated `Content` type_checker implementing [`NodeContent`]
///   that holds semantic data.
/// - Default methods like `try_parent` and `try_child` return proper errors
///   instead of panicking.
/// - Nodes are referenced via `NodeId` handled within an arena.
///
/// # See Also
///
/// - [`ArenaTree`] for managing node trees implementing this trait.
/// - [`NodeContent`] for semantic content types.
/// - [`ArenaError`] for error handling.
/// - [`SymbolRef`] for symbol references.
///
pub trait ArenaNode: Clone + Debug {
    /// Returns the ID of the parent node, if any.
    ///
    /// # Returns
    ///
    /// - `Some(NodeId)` if the node has a parent.
    /// - `None` if it is the root node.
    fn parent(&self) -> Option<NodeId>;

    /// Returns the ID of the parent node, or an error if none exists.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::MissingParent`] if the node is a root (no parent).
    ///
    /// # Example
    ///
    /// ```rust
    /// # fn example<Syntax, Arena>(syntax: &Syntax, arena: &Arena) -> Result<(), ArenaError>
    /// # where Syntax: ArenaNode, Arena: ArenaTrait<Syntax> {
    /// let parent_id = syntax.try_parent()?;
    /// let parent_node = arena.try_node(parent_id)?;
    /// println!("Parent node ID: {:?}", parent_id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// This method never panics; it returns a [`Result`].
    fn try_parent(&self) -> Result<NodeId, ArenaError> {
        self.parent().ok_or_else(ArenaError::missing_parent)
    }

    /// Sets the parent node of this syntax node.
    ///
    /// # Arguments
    ///
    /// - `parent`: The parent node’s ID, or `None` to make this node a root.
    fn set_parent(&mut self, parent: Option<NodeId>);

    /// Returns a slice of immediate child node IDs.
    ///
    /// # Returns
    ///
    /// A slice of `NodeId` representing the children of this node.
    fn children(&self) -> &[NodeId];

    /// Returns a mutable slice of immediate child node IDs.
    ///
    /// # Returns
    ///
    /// A mutable slice of `NodeId` allowing modification of this node's children.
    fn children_mut(&mut self) -> &mut Vec<NodeId>;

    /// Replaces the current children with the given list.
    ///
    /// # Example
    ///
    /// ```rust
    /// # fn example<Syntax>(syntax: &mut Syntax, child1: NodeId, child2: NodeId)
    /// # where Syntax: ArenaNode {
    /// syntax.set_children(vec![child1, child2]);
    /// # }
    /// ```
    fn set_children(&mut self, children: Vec<NodeId>);

    /// Adds a child node to this syntax node.
    ///
    /// # Arguments
    ///
    /// - `child`: The child node’s ID to add.
    fn add_child(&mut self, child: NodeId);

    /// Returns the ID of the child at the given index or an error if out of bounds.
    ///
    /// # Arguments
    ///
    /// - `index` - The zero-based child index to retrieve.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::ChildIndexOutOfBounds`] if the index is out of range.
    ///
    /// # Example
    ///
    /// ```rust
    /// # fn example<Syntax, Arena>(syntax: &Syntax, arena: &Arena) -> Result<(), ArenaError>
    /// # where Syntax: ArenaNode, Arena: ArenaTrait<Syntax> {
    /// let child_id = syntax.try_child(0)?;
    /// let child_node = arena.try_node(child_id)?;
    /// println!("Child node ID: {:?}", child_id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Never panics; returns a [`Result`].
    fn try_child(&self, index: usize) -> Result<NodeId, ArenaError> {
        self.children()
            .get(index)
            .copied()
            .ok_or_else(|| ArenaError::child_index_out_of_bounds(index, self.children().len()))
    }

    /// Returns the child ID at the given index or `None` if out of bounds.
    ///
    /// # Example
    ///
    /// ```rust
    /// # fn example<Syntax, Arena>(node: &Syntax, arena: &Arena) -> Result<(), ArenaError>
    /// # where Syntax: ArenaNode, Arena: ArenaTrait<Syntax> {
    /// if let Some(child_id) = node.get_child(0) {
    ///     let child_node = arena.try_node(child_id)?;
    ///     println!("Child node ID: {:?}", child_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn get_child(&self, index: usize) -> Option<NodeId> {
        self.children().get(index).copied()
    }

    /// Returns the number of children (arity) this node has.
    ///
    /// # Returns
    ///
    /// The count of immediate child nodes.
    ///
    /// # Example
    ///
    /// ```rust
    /// # fn example<Syntax>(node: &Syntax)
    /// # where Syntax: ArenaNode {
    /// let num_children = node.arity();
    /// println!("Number of children: {}", num_children);
    /// # }
    /// ```
    fn arity(&self) -> usize {
        self.children().len()
    }
}
