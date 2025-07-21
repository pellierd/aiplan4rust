use serde::{Deserialize, Serialize};
use crate::aiplan4rust::core::arena::NodeId;

/// A generic arena syntax used in arena-based trees.
///
/// `AbstractNode` represents a single syntax in a arena and stores its kind, content,
/// list of child nodes, and an optional parent reference.
///
/// This structure is useful for building abstract syntax trees (ASTs),
/// semantic trees, and other hierarchical data structures.
///
/// # Type Parameters
/// - `K`: A `Copy` type representing the kind of the syntax (e.g., an enum of syntax types).
/// - `C`: The content stored in the syntax. Must implement the [`NodeContent`] trait.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct BaseNode {
    children: Vec<NodeId>,
    parent: Option<NodeId>,
}

impl BaseNode {
    /// Creates a new syntax with the given kind, content, and optional parent.
    ///
    /// # Arguments
    /// - `kind`: The kind of the syntax.
    /// - `content`: The syntax's associated content.
    /// - `parent`: An optional `NodeId` referencing the parent syntax.
    ///
    /// # Returns
    /// A new `AbstractNode` instance.
    pub fn new(children: Vec<NodeId>, parent: Option<NodeId>) -> Self {
        Self {
            children,
            parent,
        }
    }

    /// Returns a slice of the child syntax IDs.
    pub fn children(&self) -> &[NodeId] {
        &self.children
    }

    /// Returns a mutable reference to the vector of child syntax IDs.
    pub fn children_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.children
    }

    /// Sets the immediate children of this syntax to the given list of syntax IDs.
    ///
    /// This replaces the current children with the provided list.
    ///
    /// # Parameters
    ///
    /// - `children`: A vector of `NodeId` that will replace the current children.
    ///
    /// # Example
    ///
    /// ```
    /// syntax.set_children(vec![child1, child2]);
    /// ```
    pub fn set_children(&mut self, children: Vec<NodeId>) {
        self.children = children;
    }

    /// Returns the parent syntax ID, if available.
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// Sets the parent syntax ID.
    ///
    /// # Arguments
    /// - `parent`: An optional new parent ID.
    pub fn set_parent(&mut self, parent: Option<NodeId>) {
        self.parent = parent;
    }

    /// Adds a child syntax ID to the list of children.
    ///
    /// # Arguments
    /// - `child`: The `NodeId` of the child to add.
    pub fn add_child(&mut self, child: NodeId) {
        self.children.push(child);
    }


}
