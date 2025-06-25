use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::arena::Arena;
use crate::aiplan4rust::arena::NodeTrait;

pub struct PreorderIter<'a, T: NodeTrait> {
    arena: &'a Arena<T>,
    stack: Vec<NodeId>,
}

impl<'a, T: NodeTrait> PreorderIter<'a, T> {
    pub fn new(arena: &'a Arena<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![root],
        }
    }
}

impl<'a, T: NodeTrait> Iterator for PreorderIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        let node = self.arena.get_node(id)?;

        for &child in node.children().iter().rev() {
            self.stack.push(child);
        }

        Some(node)
    }
}
