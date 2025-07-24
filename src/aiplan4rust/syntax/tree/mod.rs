pub mod node;
pub mod content;
pub mod tree;
pub mod base_node;
pub mod error;
pub mod subtree;

pub use tree::SyntaxTree;
pub use subtree::SyntaxSubtree;
pub use node::SyntaxNode;
pub use content::SyntaxContent;
pub use base_node::SyntaxBaseNode;

/// Type aliases for decoupling the generic `ArenaTree` module from syntax-specific usage.
///
/// These aliases are intended to provide a clearer API when using `ArenaTree`
/// for representing syntax trees, without tying the syntax module directly
/// to the low-level `core::arena` implementation.
///
/// This abstraction helps avoid unnecessary coupling between the generic arena
/// logic and language-specific structures.
///
/// - `NodeId` is an alias for the generic arena node identifier.
/// - `NodeRef` is an immutable reference to a node within the arena.
/// - `NodeRefMut` is a mutable reference to a node within the arena.
pub type NodeId = crate::aiplan4rust::core::arena::NodeId;
pub type NodeRef<'a, T> = crate::aiplan4rust::core::arena::NodeRef<'a, T>;
pub type NodeRefMut<'a, T> = crate::aiplan4rust::core::arena::node_ref::NodeRefMut<'a, T>;
