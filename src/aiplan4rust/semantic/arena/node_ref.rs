use std::fmt;
use crate::aiplan4rust::semantic::arena::{ArenaAstNode, NodeId};

#[derive(Copy, Clone, Debug)]
pub struct NodeRef<'a> {
    id: NodeId,
    node: &'a ArenaAstNode,
}

impl<'a> NodeRef<'a> {
    pub fn new(id: NodeId, node: &'a ArenaAstNode) -> Self {
        Self { id, node }
    }

    pub fn node(&self) -> &ArenaAstNode {
        self.node
    }

    pub fn id(&self) -> NodeId {
        self.id
    }
}

impl<'a> fmt::Display for NodeRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeRef {{ id: {:?}, node: {} }}", self.id, self.node)
    }
}
