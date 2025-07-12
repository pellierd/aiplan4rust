use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::arena::Arena;
use crate::aiplan4rust::arena::ArenaNode;

/// An iterator for traversing nodes in an `Arena` in preorder.
///
/// Preorder traversal visits the current syntax before its children,
/// recursively from left to right.
///
/// This iterator yields references to nodes of type `T` stored in the arena,
/// starting from a specified root syntax.
///
/// # Example
///
/// ```rust
/// let iter = PreorderIter::new(&arena, root_id);
/// for syntax in iter {
///     // Process syntax
/// }
/// ```
pub struct PreorderIter<'a, T: ArenaNode> {
    arena: &'a Arena<T>,
    stack: Vec<NodeId>,
}

impl<'a, T: ArenaNode> PreorderIter<'a, T> {
    /// Creates a new preorder iterator starting from `root`.
    ///
    /// # Parameters
    ///
    /// * `arena` - Reference to the arena containing the arena nodes.
    /// * `root` - The root syntax ID where traversal begins.
    ///
    /// # Returns
    ///
    /// A `PreorderIter` that will traverse the arena in preorder.
    pub fn new(arena: &'a Arena<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![root],
        }
    }

    pub fn empty(arena: &'a Arena<T>) -> Self {
        PreorderIter {
            arena,
            stack: Vec::new(),
        }
    }
}

impl<'a, T: ArenaNode> Iterator for PreorderIter<'a, T> {
    type Item = &'a T;

    /// Advances the iterator and returns the next syntax in preorder.
    ///
    /// The traversal order is: current syntax, then recursively each child from left to right.
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
