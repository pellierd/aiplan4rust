use crate::aiplan4rust::tree::{TreeArena, NodeId, TreeNode};

/// A preorder iterator over an `Arena` that yields node IDs along with references to the nodes.
///
/// This iterator traverses the tree starting from a given root node,
/// visiting each node before its children (preorder traversal).
/// It returns a tuple containing the `NodeId` and a reference to the node itself.
///
/// # Example
///
/// ```rust
/// let iter = PreorderIterWithIndex::new(&arena, root_id);
/// for (id, node) in iter {
///     // Process node with its id
/// }
/// ```
pub struct PreorderIterWithIndex<'a, T: TreeNode> {
    arena: &'a TreeArena<T>,
    stack: Vec<NodeId>,
}

impl<'a, T: TreeNode> PreorderIterWithIndex<'a, T> {
    /// Creates a new preorder iterator starting from the specified root node.
    ///
    /// # Parameters
    ///
    /// * `tree` - Reference to the tree containing the nodes.
    /// * `root` - The root node ID where traversal begins.
    ///
    /// # Returns
    ///
    /// A `PreorderIterWithIndex` instance ready to traverse the tree.
    pub fn new(arena: &'a TreeArena<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![root],
        }
    }
}

impl<'a, T: TreeNode> Iterator for PreorderIterWithIndex<'a, T> {
    type Item = (NodeId, &'a T);

    /// Returns the next node ID and reference in preorder traversal order.
    ///
    /// Visits the current node first, then pushes its children onto the stack
    /// in reverse order to maintain left-to-right traversal.
    ///
    /// Returns `None` when all nodes have been visited.
    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        let node = self.arena.get_node(id)?;

        // Push children in reverse order for left-to-right traversal
        for &child_id in node.children().iter().rev() {
            self.stack.push(child_id);
        }

        Some((id, node))
    }
}
