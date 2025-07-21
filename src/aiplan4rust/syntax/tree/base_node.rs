use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::core::arena::{ArenaNode, BaseNode, NodeId};
use crate::aiplan4rust::syntax::tree::SyntaxContent;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SyntaxBaseNode<K: Copy + Debug + Display, C: SyntaxContent> {
    base_node: BaseNode,
    kind: K,
    content: C,
}

impl<K: Copy + Debug + Display, C: SyntaxContent> SyntaxBaseNode<K, C> {
    pub fn new(kind: K, content: C, children: Vec<NodeId>, parent: Option<NodeId>) -> Self {
        Self {
            base_node: BaseNode::new(children, parent),
            kind,
            content,
        }
    }

    /// Returns the kind of the syntax.
    pub fn kind(&self) -> K {
        self.kind
    }

    /// Sets the kind of the syntax.
    ///
    /// # Arguments
    /// - `kind`: The new kind to assign to the syntax.
    pub fn set_kind(&mut self, kind: K) {
        self.kind = kind;
    }

    /// Returns an immutable reference to the syntax's content.
    pub fn content(&self) -> &C {
        &self.content
    }

    /// Returns a mutable reference to the syntax's content.
    pub fn content_mut(&mut self) -> &mut C {
        &mut self.content
    }

    /// Replaces the content of the syntax.
    ///
    /// # Arguments
    /// - `content`: The new content to assign to the syntax.
    pub fn set_content(&mut self, content: C) {
        self.content = content;
    }
}

impl<K: Copy + Debug + Display, C: SyntaxContent> Deref for SyntaxBaseNode<K, C> {
    type Target = BaseNode;

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
