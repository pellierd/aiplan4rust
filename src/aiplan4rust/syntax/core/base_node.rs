use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::arena::{ArenaNode, BaseNode, NodeId};
use crate::aiplan4rust::syntax::core::SyntaxContent;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SyntaxBaseNode<K: Copy + Debug + Display, C: SyntaxContent> {
    base_node: BaseNode<K, C>,
}

impl<K: Copy + Debug + Display, C: SyntaxContent> SyntaxBaseNode<K, C> {
    pub fn new(kind: K, content: C, children: Vec<NodeId>, parent: Option<NodeId>) -> Self {
        Self {
            base_node: BaseNode::<K, C>::new(kind, content, children, parent),
        }
    }
}

impl<K: Copy + Debug + Display, C: SyntaxContent> Deref for SyntaxBaseNode<K, C> {
    type Target = BaseNode<K, C>;

    fn deref(&self) -> &Self::Target {
        &self.base_node
    }
}

impl<K: Copy + Debug + Display, C: SyntaxContent> DerefMut for SyntaxBaseNode<K, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_node
    }
}


impl<K: Copy + Debug + Display, C: SyntaxContent> ArenaNode for SyntaxBaseNode<K, C> {
    type Kind = K;
    type Content = C;

    fn kind(&self) -> Self::Kind {
        self.base_node.kind()
    }

    fn set_kind(&mut self, kind: Self::Kind) {
        self.base_node.set_kind(kind)
    }

    fn content(&self) -> &Self::Content {
        self.base_node.content()
    }

    fn content_mut(&mut self) -> &mut Self::Content {
        self.base_node.content_mut()
    }

    fn parent(&self) -> Option<NodeId> {
        self.base_node.parent()
    }

    fn set_parent(&mut self, parent: Option<NodeId>) {
        self.base_node.set_parent(parent)
    }

    fn children(&self) -> &[NodeId] {
        self.base_node.children()
    }

    fn set_children(&mut self, children: Vec<NodeId>) {
        self.base_node.set_children(children)
    }

    fn add_child(&mut self, child: NodeId) {
        self.base_node.add_child(child)
    }
}
