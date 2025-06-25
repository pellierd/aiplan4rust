use crate::aiplan4rust::arena::arena::Arena;
use crate::aiplan4rust::arena::node::NodeTrait;
use crate::aiplan4rust::arena::NodeId;

/// Postorder (depth-first) iterator over `Arena` nodes.
pub struct PostorderIter<'a, T: NodeTrait> {
    arena: &'a Arena<T>,
    stack: Vec<(NodeId, usize)>,
}

impl<'a, T: NodeTrait> PostorderIter<'a, T> {
    /// Create a postorder iterator starting from `root`.
    pub fn new(arena: &'a Arena<T>, root: NodeId) -> Self {
        PostorderIter { arena, stack: vec![(root, 0)] }
    }
}

impl<'a, T: NodeTrait> Iterator for PostorderIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((id, idx)) = self.stack.pop() {
            let node = self.arena.get_node(id)?;
            let children = node.children();

            if idx < children.len() {
                // Push current node with incremented index for next visit
                self.stack.push((id, idx + 1));
                // Push the current child
                self.stack.push((children[idx], 0));
            } else {
                // All children visited, return node
                return Some(node);
            }
        }
        None
    }
}
