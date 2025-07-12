use crate::aiplan4rust::arena::arena::Arena;
use crate::aiplan4rust::arena::node::ArenaNode;
use crate::aiplan4rust::arena::NodeId;

/// An iterator that traverses the nodes of an `Arena` in postorder (depth-first).
///
/// Postorder traversal means all children of a node are visited before the node itself.
///
/// The iterator yields references to nodes of type `T` that implement the `TreeNode` trait.
///
/// # Example
///
/// ```rust
/// let iter = PostorderIter::new(&arena, root_id);
/// for node in iter {
///     // Process node
/// }
/// ```
pub struct PostorderIter<'a, T: ArenaNode> {
    arena: &'a Arena<T>,
    stack: Vec<(NodeId, usize)>,
}

impl<'a, T: ArenaNode> PostorderIter<'a, T> {
    /// Creates a new postorder iterator starting from the given `root` node ID.
    ///
    /// # Parameters
    ///
    /// * `arena` - Reference to the arena containing the nodes.
    /// * `root` - The root node ID to start traversal from.
    ///
    /// # Returns
    ///
    /// A `PostorderIter` instance that will traverse the arena rooted at `root`.
    pub fn new(arena: &'a Arena<T>, root: NodeId) -> Self {
        PostorderIter { arena, stack: vec![(root, 0)] }
    }

    /// Creates an empty postorder iterator with no nodes.
    pub fn empty(arena: &'a Arena<T>) -> Self {
        PostorderIter { arena, stack: Vec::new() }
    }
}

impl<'a, T: ArenaNode> Iterator for PostorderIter<'a, T> {
    type Item = &'a T;

    /// Returns the next node in postorder traversal.
    ///
    /// Visits all children of a node before the node itself.
    ///
    /// Returns `None` when all nodes have been traversed.
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((id, idx)) = self.stack.pop() {
            let node = self.arena.get_node(id)?;
            let children = node.children();

            if idx < children.len() {
                // Push the current node with incremented child index for future visits
                self.stack.push((id, idx + 1));
                // Push the child node to be visited next (depth-first)
                self.stack.push((children[idx], 0));
            } else {
                // All children have been visited, now yield the current node
                return Some(node);
            }
        }
        None
    }
}
