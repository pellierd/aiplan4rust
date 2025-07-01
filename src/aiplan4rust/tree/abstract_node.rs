use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::tree::{NodeContent, NodeId};

/// A generic tree node used in arena-based trees.
///
/// `AbstractNode` represents a single node in a tree and stores its kind, content,
/// list of child nodes, and an optional parent reference.
///
/// This structure is useful for building abstract syntax trees (ASTs),
/// semantic trees, and other hierarchical data structures.
///
/// # Type Parameters
/// - `K`: A `Copy` type representing the kind of the node (e.g., an enum of node types).
/// - `C`: The content stored in the node. Must implement the [`NodeContent`] trait.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AbstractNode<K: Copy, C: NodeContent> {
    kind: K,
    content: C,
    children: Vec<NodeId>,
    parent: Option<NodeId>,
}

impl<K: Copy, C: NodeContent> AbstractNode<K, C> {
    /// Creates a new node with the given kind, content, and optional parent.
    ///
    /// # Arguments
    /// - `kind`: The kind of the node.
    /// - `content`: The node's associated content.
    /// - `parent`: An optional `NodeId` referencing the parent node.
    ///
    /// # Returns
    /// A new `AbstractNode` instance.
    pub fn new(kind: K, content: C, parent: Option<NodeId>) -> Self {
        Self {
            kind,
            content,
            children: Vec::new(),
            parent,
        }
    }

    /// Returns the kind of the node.
    pub fn kind(&self) -> K {
        self.kind
    }

    /// Sets the kind of the node.
    ///
    /// # Arguments
    /// - `kind`: The new kind to assign to the node.
    pub fn set_kind(&mut self, kind: K) {
        self.kind = kind;
    }

    /// Returns an immutable reference to the node's content.
    pub(crate) fn content(&self) -> &C {
        &self.content
    }

    /// Returns a mutable reference to the node's content.
    pub fn content_mut(&mut self) -> &mut C {
        &mut self.content
    }

    /// Replaces the content of the node.
    ///
    /// # Arguments
    /// - `content`: The new content to assign to the node.
    pub fn set_content(&mut self, content: C) {
        self.content = content;
    }

    /// Returns a slice of the child node IDs.
    pub fn children(&self) -> &[NodeId] {
        &self.children
    }

    /// Returns a mutable reference to the vector of child node IDs.
    pub fn children_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.children
    }

    /// Returns the parent node ID, if available.
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// Sets the parent node ID.
    ///
    /// # Arguments
    /// - `parent`: An optional new parent ID.
    pub fn set_parent(&mut self, parent: Option<NodeId>) {
        self.parent = parent;
    }

    /// Adds a child node ID to the list of children.
    ///
    /// # Arguments
    /// - `child`: The `NodeId` of the child to add.
    pub fn add_child(&mut self, child: NodeId) {
        self.children.push(child);
    }

    /// Applies identifier remapping to the content of the node using the provided map.
    ///
    /// This is a generic wrapper that delegates to the content's own remap_idents method.
    ///
    /// # Arguments
    /// * `map` - A mapping from old identifiers to new ones.
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        self.content.remap_idents(map);
    }
}

impl<K, C> DisplayWithInterner for AbstractNode<K, C>
where
    K: Copy + fmt::Display,
    C: NodeContent + DisplayWithInterner,
{
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        // Affiche `kind` avec Debug et `content` avec DisplayWithInterner
        write!(
            f,
            "Node[kind={}, content=",
            self.kind
        )?;

        self.content.fmt_with(f, interner)?;

        write!(
            f,
            "parent={:?}]",
            self.parent
        )
    }
}
