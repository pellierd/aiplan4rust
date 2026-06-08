pub mod base_node;
pub mod builder;
pub mod content;
pub mod error;
/// This module provides common abstractions and types for syntax tree management.
///
/// It includes definitions for syntax nodes, content, tree structures,
/// and error handling, all built on top of a generic arena-based storage
/// system to efficiently manage tree nodes.
///
/// The module exports key traits and types such as [`Node`], [`SyntaxContent`],
/// [`Tree`], and error types, enabling construction and manipulation
/// of syntax trees representing language constructs.
///
/// It also defines typing aliases to decouple syntax-specific code
/// from the underlying arena implementation, improving modularity and clarity.
///
/// The following typing aliases simplification the API by abstracting the generic
/// `ArenaTree` types for syntax-specific usage:
/// - `NodeId`: Unique node identifier.
/// - `NodeRef<'a, T>`: Immutable node reference.
/// - `NodeRefMut<'a, T>`: Mutable node reference.
pub mod node;
pub mod subtree;
pub mod tree;

pub use base_node::SyntaxBaseNode;
pub use builder::SyntaxTreeBuilder;
pub use content::SyntaxContent;
pub use node::Node;
pub use subtree::SyntaxSubtree;
pub use tree::Tree;

pub type NodeId = crate::aiplan4rust::syntax::ast::arena::NodeId;
pub type NodeRef<'a, T> = crate::aiplan4rust::syntax::ast::arena::NodeRef<'a, T>;
pub type NodeRefMut<'a, T> = crate::aiplan4rust::syntax::ast::arena::node_ref::NodeRefMut<'a, T>;
