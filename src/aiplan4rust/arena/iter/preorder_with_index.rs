use crate::aiplan4rust::arena::{Arena, NodeId, ArenaNode};

/// A preorder iterator over an `Arena` that yields syntax IDs along with references to the nodes.
///
/// This iterator traverses the arena starting from a given root syntax,
/// visiting each syntax before its children (preorder traversal).
/// It returns a tuple containing the `NodeId` and a reference to the syntax itself.
///
/// # Example
///
/// ```rust
/// let iter = PreorderIterWithIndex::new(&arena, root_id);
/// for (id, syntax) in iter {
///     // Process syntax with its id
/// }
/// ```
pub struct PreorderIterWithIndex<'a, T: ArenaNode> {
    arena: &'a Arena<T>,
    stack: Vec<NodeId>,
}

impl<'a, T: ArenaNode> PreorderIterWithIndex<'a, T> {
    /// Creates a new preorder iterator starting from the specified root syntax.
    ///
    /// # Parameters
    ///
    /// * `arena` - Reference to the arena containing the nodes.
    /// * `root` - The root syntax ID where traversal begins.
    ///
    /// # Returns
    ///
    /// A `PreorderIterWithIndex` instance ready to traverse the arena.
    pub fn new(arena: &'a Arena<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![root],
        }
    }

    /// Creates an empty preorder iterator with no nodes.
    pub fn empty(arena: &'a Arena<T>) -> Self {
        Self {
            arena,
            stack: Vec::new(),
        }
    }
}

impl<'a, T: ArenaNode> Iterator for PreorderIterWithIndex<'a, T> {
    type Item = (NodeId, &'a T);

    /// Returns the next syntax ID and reference in preorder traversal order.
    ///
    /// Visits the current syntax first, then pushes its children onto the stack
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
