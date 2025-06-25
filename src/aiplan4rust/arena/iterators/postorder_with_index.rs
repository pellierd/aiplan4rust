use crate::aiplan4rust::arena::{Arena, NodeId, NodeTrait};

/// Postorder iterator over a generic `Arena`, yielding `(Id, &T)` pairs.
pub struct PostorderIterWithIndex<'a, T: NodeTrait> {
    arena: &'a Arena<T>,
    stack: Vec<(NodeId, bool)>, // (node id, children visited)
}

impl<'a, T: NodeTrait> PostorderIterWithIndex<'a, T> {
    /// Creates a postorder iterator starting from `root`.
    pub fn new(arena: &'a Arena<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![(root, false)],
        }
    }
}

impl<'a, T: NodeTrait> Iterator for PostorderIterWithIndex<'a, T> {
    type Item = (NodeId, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&(id, visited)) = self.stack.last() {
            if !visited {
                // Mark node as visited
                if let Some(top) = self.stack.last_mut() {
                    top.1 = true;
                }

                // Push children in reverse to preserve left-to-right traversal
                if let Some(node) = self.arena.get_node(id) {
                    for &child_id in node.children().iter().rev() {
                        self.stack.push((child_id, false));
                    }
                }
            } else {
                // All children visited, yield node
                self.stack.pop();
                if let Some(node) = self.arena.get_node(id) {
                    return Some((id, node));
                }
            }
        }
        None
    }
}
