use crate::aiplan4rust::tree::{NodeId, Arena, ArenaNode};

/// An iterator for traversing a tree in preorder, yielding only `NodeId`s.
///
/// This is useful when you want to perform both immutable and mutable access
/// using `NodeId` later.
///
/// # Example
/// ```rust
/// let iter = PreorderIdIter::new(&arena, root_id);
/// for node_id in iter {
///     let node = arena.get_node(node_id).unwrap();
///     // Process node
/// }
/// ```
pub struct PreorderIdIter<'a, T: ArenaNode> {
    arena: &'a Arena<T>,
    stack: Vec<NodeId>,
}

impl<'a, T: ArenaNode> PreorderIdIter<'a, T> {
    /// Creates a new preorder iterator starting from `root`.
    pub fn new(arena: &'a Arena<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![root],
        }
    }

    /// Creates an empty iterator (useful for no-op cases).
    pub fn empty(arena: &'a Arena<T>) -> Self {
        Self {
            arena,
            stack: Vec::new(),
        }
    }
}

impl<'a, T: ArenaNode> Iterator for PreorderIdIter<'a, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        let node = self.arena.get_node(id)?;

        // Push children in reverse order so left-to-right traversal
        for &child_id in node.children().iter().rev() {
            self.stack.push(child_id);
        }

        Some(id)
    }
}
