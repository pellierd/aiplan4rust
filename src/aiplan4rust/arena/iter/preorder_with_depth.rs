use crate::aiplan4rust::arena::{NodeId, Arena, ArenaNode};

/// Preorder iterator yielding (NodeId, &T, depth)
pub struct PreorderIterWithDepth<'a, T: ArenaNode> {
    arena: &'a Arena<T>,
    stack: Vec<(NodeId, usize)>,
}

impl<'a, T: ArenaNode> PreorderIterWithDepth<'a, T> {
    /// Creates a new iterator starting from the given root node.
    pub fn new(arena: &'a Arena<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![(root, 0)],
        }
    }

    /// Creates an empty iterator (no nodes to iterate).
    pub fn empty(arena: &'a Arena<T>) -> Self {
        Self {
            arena,
            stack: Vec::new(),
        }
    }
}

impl<'a, T: ArenaNode> Iterator for PreorderIterWithDepth<'a, T> {
    type Item = (NodeId, &'a T, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let (id, depth) = self.stack.pop()?;
        let node = self.arena.get_node(id)?;

        // Push children in reverse order to maintain left-to-right traversal
        for &child in node.children().iter().rev() {
            self.stack.push((child, depth + 1));
        }

        Some((id, node, depth))
    }
}
