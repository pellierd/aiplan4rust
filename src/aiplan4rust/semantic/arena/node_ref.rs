use std::fmt;
use crate::aiplan4rust::semantic::arena::{ArenaAstNode, NodeId};

/// A lightweight reference to an AST node stored in an arena.
///
/// `NodeRef` holds a reference to a node (`ArenaAstNode`) along with its
/// unique identifier (`NodeId`). This struct allows easy access to the node
/// and its ID without ownership, enabling safe and efficient traversal or
/// manipulation of AST nodes within the arena.
///
/// The lifetime `'a` ensures the `NodeRef` cannot outlive the arena node it references.
#[derive(Copy, Clone, Debug)]
pub struct NodeRef<'a> {
    id: NodeId,
    node: &'a ArenaAstNode,
}

impl<'a> NodeRef<'a> {
    /// Creates a new `NodeRef` from a node ID and a reference to an arena node.
    ///
    /// # Parameters
    ///
    /// - `id`: The unique identifier of the node.
    /// - `node`: A reference to the actual `ArenaAstNode` in the arena.
    ///
    /// # Returns
    ///
    /// A new `NodeRef` instance encapsulating the provided ID and node reference.
    pub fn new(id: NodeId, node: &'a ArenaAstNode) -> Self {
        Self { id, node }
    }

    /// Returns a reference to the underlying `ArenaAstNode`.
    ///
    /// # Returns
    ///
    /// A reference to the arena node this `NodeRef` points to.
    pub fn node(&self) -> &ArenaAstNode {
        self.node
    }

    /// Returns the unique identifier of the node.
    ///
    /// # Returns
    ///
    /// The `NodeId` associated with this `NodeRef`.
    pub fn id(&self) -> NodeId {
        self.id
    }
}

/// Allows conversion from a [`NodeRef`] reference to its corresponding [`NodeId`].
///
/// This implementation enables seamless usage of functions that take a `NodeId`
/// while passing a reference to a `NodeRef`, improving API ergonomics.
///
/// # Example
///
/// ```rust
/// fn get_symbol<T: Into<NodeId>>(input: T) { /* ... */ }
///
/// let node_ref: NodeRef = /* obtain a NodeRef */;
/// let _ = get_symbol(&node_ref); // Works because of this `From` implementation
/// ```
impl<'a> From<&NodeRef<'a>> for NodeId {
    fn from(node_ref: &NodeRef<'a>) -> Self {
        node_ref.id()
    }
}

impl<'a> fmt::Display for NodeRef<'a> {
    /// Formats the `NodeRef` for display purposes.
    ///
    /// This implementation prints the node ID using its debug format and the node
    /// itself using its display implementation.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let node_ref = NodeRef::new(id, &node);
    /// println!("{}", node_ref);
    /// // Output: NodeRef { id: NodeId(1), node: ... }
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeRef {{ id: {:?}, node: {} }}", self.id, self.node)
    }
}
