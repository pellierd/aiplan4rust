use crate::aiplan4rust::arena::{Arena, NodeId, Node};

/// Un itérateur en pré-ordre qui retourne l'identifiant et une référence au nœud.
pub struct PreorderIterWithIndex<'a, T: Node> {
    arena: &'a Arena<T>,
    stack: Vec<NodeId>,
}

impl<'a, T: Node> PreorderIterWithIndex<'a, T> {
    /// Crée un nouvel itérateur à partir d'une arène et d'une racine.
    pub fn new(arena: &'a Arena<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![root],
        }
    }
}

impl<'a, T: Node> Iterator for PreorderIterWithIndex<'a, T> {
    type Item = (NodeId, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        let node = self.arena.get_node(id)?;

        // Empile les enfants en ordre inverse pour traverser de gauche à droite
        for &child_id in node.children().iter().rev() {
            self.stack.push(child_id);
        }

        Some((id, node))
    }
}
