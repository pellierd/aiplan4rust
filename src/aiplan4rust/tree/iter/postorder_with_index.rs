use crate::aiplan4rust::tree::{TreeArena, NodeId, TreeNode};

/// A postorder iterator over nodes in an `Arena`, yielding `(NodeId, &T)` pairs.
///
/// This iterator traverses the tree in postorder, meaning it visits all children of a node
/// before the node itself. Each iteration returns the unique node identifier along with
/// a reference to the node.
///
/// The traversal preserves left-to-right order by pushing children onto the stack in reverse.
///
/// # Example
///
/// ```rust
/// let iter = PostorderIterWithIndex::new(&arena, root_id);
/// for (id, node) in iter {
///     // Process node with its id
/// }
/// ```
pub struct PostorderIterWithIndex<'a, T: TreeNode> {
    arena: &'a TreeArena<T>,
    stack: Vec<(NodeId, bool)>, // (node id, children visited flag)
}

impl<'a, T: TreeNode> PostorderIterWithIndex<'a, T> {
    /// Creates a new postorder iterator starting from the specified root node.
    ///
    /// # Parameters
    ///
    /// * `tree` - Reference to the tree containing the tree nodes.
    /// * `root` - The root node ID from which to start traversal.
    ///
    /// # Returns
    ///
    /// A `PostorderIterWithIndex` ready to traverse the tree in postorder.
    pub fn new(arena: &'a TreeArena<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![(root, false)],
        }
    }

    /// Creates an empty postorder iterator with no nodes.
    pub fn empty(arena: &'a TreeArena<T>) -> Self {
        Self {
            arena,
            stack: Vec::new(),
        }
    }
}

impl<'a, T: TreeNode> Iterator for PostorderIterWithIndex<'a, T> {
    type Item = (NodeId, &'a T);

    /// Returns the next node in postorder traversal along with its ID.
    ///
    /// The traversal visits all children of a node before the node itself.
    ///
    /// Returns `None` when traversal is complete.
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&(id, visited)) = self.stack.last() {
            if !visited {
                // Mark this node as visited to indicate children will be processed
                if let Some(top) = self.stack.last_mut() {
                    top.1 = true;
                }

                // Push children in reverse order to traverse them left-to-right
                if let Some(node) = self.arena.get_node(id) {
                    for &child_id in node.children().iter().rev() {
                        self.stack.push((child_id, false));
                    }
                }
            } else {
                // All children visited; yield this node
                self.stack.pop();
                if let Some(node) = self.arena.get_node(id) {
                    return Some((id, node));
                }
            }
        }
        None
    }
}
