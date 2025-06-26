use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::tree::TreeArena;
use crate::aiplan4rust::tree::TreeNode;

/// An iterator for traversing nodes in an `Arena` in preorder.
///
/// Preorder traversal visits the current node before its children,
/// recursively from left to right.
///
/// This iterator yields references to nodes of type `T` stored in the tree,
/// starting from a specified root node.
///
/// # Example
///
/// ```rust
/// let iter = PreorderIter::new(&tree, root_id);
/// for node in iter {
///     // Process node
/// }
/// ```
pub struct PreorderIter<'a, T: TreeNode> {
    arena: &'a TreeArena<T>,
    stack: Vec<NodeId>,
}

impl<'a, T: TreeNode> PreorderIter<'a, T> {
    /// Creates a new preorder iterator starting from `root`.
    ///
    /// # Parameters
    ///
    /// * `tree` - Reference to the tree containing the tree nodes.
    /// * `root` - The root node ID where traversal begins.
    ///
    /// # Returns
    ///
    /// A `PreorderIter` that will traverse the tree in preorder.
    pub fn new(arena: &'a TreeArena<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![root],
        }
    }
}

impl<'a, T: TreeNode> Iterator for PreorderIter<'a, T> {
    type Item = &'a T;

    /// Advances the iterator and returns the next node in preorder.
    ///
    /// The traversal order is: current node, then recursively each child from left to right.
    ///
    /// Returns `None` when all nodes have been visited.
    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        let node = self.arena.get_node(id)?;

        // Push children in reverse order so the leftmost child is processed first
        for &child in node.children().iter().rev() {
            self.stack.push(child);
        }

        Some(node)
    }
}
