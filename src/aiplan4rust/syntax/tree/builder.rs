use crate::aiplan4rust::core::arena::NodeId;
use crate::aiplan4rust::syntax::tree::{SyntaxTree, SyntaxNode, SyntaxContent};
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Generic builder for [`SyntaxTree<T>`].
///
/// `SyntaxTreeBuilder` provides a convenient way to construct syntax trees
/// in an arena-based structure. It handles:
/// - Allocating nodes (leaves and internal nodes)
/// - Assigning children to nodes
/// - Setting the root node
///
/// This builder is generic over any type `T` implementing [`SyntaxNode`].
/// Concrete builders like [`ExprBuilder`] or [`AstBuilder`] can wrap this
/// generic builder to provide domain-specific helpers (e.g., `and`, `or`, `forall`).
///
/// # Examples
///
/// ## Using the builder methods directly
///
/// ```rust,ignore
/// let mut builder = SyntaxTreeBuilder::<MyNode>::new();
///
/// // Allocate leaves
/// let leaf_a = builder.leaf(MyNode::new_leaf("A"));
/// let leaf_b = builder.leaf(MyNode::new_leaf("B"));
///
/// // Create an internal node with children
/// let internal = builder.node(MyNode::new_internal("And"), vec![leaf_a, leaf_b]);
///
/// // Set root
/// builder.set_root(internal).unwrap();
///
/// // Retrieve the final tree
/// let tree = builder.finish();
/// assert_eq!(tree.root_id(), Some(internal));
/// ```
///
/// ## Using macros (more concise)
///
/// ```rust,ignore
/// let mut builder = SyntaxTreeBuilder::<MyNode>::new();
///
/// // Allocate leaves with `leaf!` macro
/// let leaf_a = leaf!(builder, MyNode::new_leaf("A"));
/// let leaf_b = leaf!(builder, MyNode::new_leaf("B"));
///
/// // Create internal node with children using `node!` macro
/// let internal = node!(builder, MyNode::new_internal("And"), [leaf_a, leaf_b]);
///
/// // Create root node directly with `root_node!` macro
/// let root = root_node!(builder, MyNode::new_internal("Or"), [leaf_a, leaf_b]);
///
/// // Retrieve the final tree
/// let tree = builder.finish();
/// assert_eq!(tree.root_id(), Some(root));
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct SyntaxTreeBuilder<T: SyntaxNode>
where
    T::Content: SyntaxContent,
{
    tree: SyntaxTree<T>,
}

impl<T: SyntaxNode> SyntaxTreeBuilder<T>
where
    T::Content: SyntaxContent,
{
    /// Creates a new, empty syntax tree builder.
    ///
    /// # Returns
    /// A new instance of `SyntaxTreeBuilder<T>` with no nodes allocated.
    pub fn new() -> Self {
        Self {
            tree: SyntaxTree::new(),
        }
    }

    /// Consumes the builder and returns the built [`SyntaxTree<T>`].
    ///
    /// # Returns
    /// The fully constructed `SyntaxTree<T>`.
    pub fn finish(self) -> SyntaxTree<T> {
        self.tree
    }

    /// Allocates a new leaf node in the tree.
    ///
    /// # Parameters
    /// * `node` - The syntax node to insert as a leaf.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly allocated node.
    pub fn leaf(&mut self, node: T) -> NodeId {
        self.tree.alloc(node)
    }

    /// Allocates a new node with specified children.
    ///
    /// # Parameters
    /// * `node` - The syntax node to insert.
    /// * `children` - A vector of [`NodeId`] representing child nodes.
    ///
    /// # Returns
    /// The [`NodeId`] of the newly allocated node.
    pub fn node(&mut self, node: T, children: Vec<NodeId>) -> NodeId {
        self.tree.alloc_with_children(node, children)
    }

    /// Sets the root of the syntax tree.
    ///
    /// # Parameters
    /// * `id` - The [`NodeId`] of the node to set as root.
    ///
    /// # Returns
    /// `Ok(&mut Self)` if successful, otherwise [`SyntaxTreeError`] if the node ID is invalid.
    pub fn set_root(&mut self, id: NodeId) -> Result<&mut Self, SyntaxTreeError> {
        self.tree.set_root_id(id)?;
        Ok(self)
    }

    /// Allocates a new node with children and sets it as the root.
    ///
    /// # Parameters
    /// * `node` - The syntax node to insert as root.
    /// * `children` - Vector of [`NodeId`] representing the children of the root node.
    ///
    /// # Returns
    /// `Ok(NodeId)` of the newly allocated root node, or [`SyntaxTreeError`] on failure.
    pub fn root_node(&mut self, node: T, children: Vec<NodeId>) -> Result<NodeId, SyntaxTreeError> {
        let id = self.tree.alloc_root_with_children(node, children);
        Ok(id)
    }

    /// Provides mutable access to the underlying syntax tree.
    ///
    /// # Returns
    /// A mutable reference to the internal [`SyntaxTree<T>`].
    pub fn tree_mut(&mut self) -> &mut SyntaxTree<T> {
        &mut self.tree
    }

    /// Provides immutable access to the underlying syntax tree.
    ///
    /// # Returns
    /// An immutable reference to the internal [`SyntaxTree<T>`].
    pub fn tree(&self) -> &SyntaxTree<T> {
        &self.tree
    }
}

/// Macro to allocate a leaf node in a [`SyntaxTreeBuilder`].
///
/// # Example
///
/// ```rust,ignore
/// let leaf_id = leaf!(builder, MyNode::new_leaf("X"));
/// ```
#[macro_export]
macro_rules! leaf {
    ($builder:expr, $node:expr) => {
        $builder.leaf($node)
    };
}

/// Macro to allocate an internal node with children.
///
/// # Example
///
/// ```rust,ignore
/// let internal_id = node!(builder, MyNode::new_internal("And"), [leaf1, leaf2]);
/// ```
#[macro_export]
macro_rules! node {
    ($builder:expr, $node:expr, [$($child:expr),* $(,)?]) => {
        $builder.node($node, vec![$($child),*])
    };
}

/// Macro to allocate a node and set it as the root, returning a [`Result<NodeId, SyntaxTreeError>`].
///
/// # Example
///
/// ```rust,ignore
/// let root_id = root_node_safe!(builder, MyNode::new_internal("Or"), [leaf1, leaf2])?;
/// ```
#[macro_export]
macro_rules! root_node_safe {
    ($builder:expr, $node:expr, [$($child:expr),* $(,)?]) => {
        $builder.root_node($node, vec![$($child),*])
    };
}
