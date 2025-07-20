use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use ordered_float::OrderedFloat;
use crate::aiplan4rust::arena::{Arena, NodeContent, NodeId};
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::symbol::SymbolRef;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, Ident, Optimization};
use crate::aiplan4rust::syntax::core::{SyntaxNode, SyntaxTree};
use crate::aiplan4rust::syntax::SyntaxDisplay;

/// A generic trait representing a syntax in a arena stored within an `Arena`.
///
/// This trait defines the minimal interface that any syntax type must implement to
/// be used in a arena structure managed by a `TreeArena`. It provides methods to
/// navigate parent-child relationships, modify the arena, work with syntax content,
/// and extract semantic information like identifiers and symbols.
///
/// # Trait Type Parameters
///
/// - `Kind`: The type representing the kind/category of the syntax.
/// - `Content`: The type of the syntax’s semantic content, which must implement [`NodeContent`].
///
/// # Core Responsibilities
///
/// - Access and modify the syntax's kind and content.
/// - Navigate the arena structure: get parent and children, add children.
/// - Remap identifiers within the syntax's content using a mapping table.
/// - Query syntax properties such as leaf/root status and arity (number of children).
/// - Attempt to extract a symbol reference from the syntax within a arena arena.
///
/// # Example
///
/// ```rust
/// use std::collections::HashMap;
/// use crate::aiplan4rust::arena::{TreeNode, NodeId};
/// use crate::aiplan4rust::syntax::elements::Ident;
///
/// struct MyNode {
///     parent: Option<NodeId>,
///     children: Vec<NodeId>,
///     idents: Vec<Ident>,
/// }
///
/// impl TreeNode for MyNode {
///     type Kind = MyKind;
///     type Content = MyContent;
///
///     fn kind(&self) -> Self::Kind { /* ... */ }
///     fn set_kind(&mut self, kind: Self::Kind) { /* ... */ }
///     fn content(&self) -> &Self::Content { /* ... */ }
///     fn content_mut(&mut self) -> &mut Self::Content { /* ... */ }
///
///     fn parent(&self) -> Option<NodeId> { self.parent }
///     fn set_parent(&mut self, parent: Option<NodeId>) { self.parent = parent; }
///     fn children(&self) -> &[NodeId] { &self.children }
///     fn add_child(&mut self, child: NodeId) { self.children.push(child); }
///
///     fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
///         for id in &mut self.idents {
///             if let Some(new_id) = map.get(id) {
///                 *id = *new_id;
///             }
///         }
///     }
///
///     fn try_symbol_ref(&self, _arena: &TreeArena<Self>) -> Result<SymbolRef, ParserInternalError> {
///         unimplemented!()
///     }
/// }
/// ```
///
/// # Notes
///
/// - The default implementations of some methods (like `is_leaf`, `arity`, and
///   delegation methods for content extraction) provide convenience but can
///   be overridden for optimization or specific behaviors.
/// - The trait expects an associated `Content` type implementing [`NodeContent`],
///   providing methods to access semantic information.
///
/// # See Also
///
/// - [`Arena`] for managing trees of nodes implementing this trait.
/// - [`NodeContent`] for content types that hold semantic syntax data.
/// - [`AiplanError`] for error handling during parsing or resolution.
/// - [`SymbolRef`] for referencing symbols resolved from nodes.
///
pub trait ArenaNode: Clone {
    /// The type used to represent the syntax's kind.
    type Kind: std::fmt::Display;

    /// The type used to represent the semantic content of the syntax.
    type Content: NodeContent;

    /// Returns the kind of the syntax.
    fn kind(&self) -> Self::Kind;

    /// Sets the kind of the syntax.
    fn set_kind(&mut self, kind: Self::Kind);

    /// Returns a reference to the syntax's semantic content.
    fn content(&self) -> &Self::Content;

    /// Returns a mutable reference to the syntax's semantic content.
    fn content_mut(&mut self) -> &mut Self::Content;

    /// Returns `true` if the syntax has no meaningful content.
    ///
    /// By default, this delegates to `content().is_none()`.
    fn has_content(&self) -> bool {
        self.content().is_none()
    }

    /// Returns the ID of the syntax’s parent if it exists.
    ///
    /// # Returns
    ///
    /// - `Some(NodeId)` if the syntax has a parent.
    /// - `None` if this syntax is the root of the arena.
    fn parent(&self) -> Option<NodeId>;

    /// Returns the `NodeId` of the parent of this syntax, or an error if there is no parent.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if this syntax has no parent.
    ///
    /// # Example
    /// ```rust
    /// let parent_id = syntax.try_parent()?;
    /// let parent_node = arena.try_node(parent_id)?;
    /// ```
    ///
    /// # Panics
    /// This method does **not** panic. It returns a proper `Result`.
    fn try_parent(&self) -> Result<NodeId, AiplanError> {
        self.parent().ok_or_else(|| AiplanError::InternalError(format!(
            "Expected parent for syntax kind {} but found none",
            self.kind()
        )))
    }

    /// Sets the parent of this syntax.
    ///
    /// # Arguments
    ///
    /// - `parent`: The parent syntax's ID, or `None` to unset and mark this syntax as root.
    fn set_parent(&mut self, parent: Option<NodeId>);

    /// Returns a slice of IDs representing this syntax’s immediate children.
    ///
    /// # Returns
    ///
    /// A slice of `NodeId` elements corresponding to the children.
    fn children(&self) -> &[NodeId];

    /// Sets the immediate children of this syntax to the given list of syntax IDs.
    ///
    /// This replaces the current children with the provided list.
    ///
    /// # Parameters
    ///
    /// - `children`: A vector of `NodeId` that will replace the current children.
    ///
    /// # Example
    ///
    /// ```
    /// syntax.set_children(vec![child1, child2]);
    /// ```
    fn set_children(&mut self, children: Vec<NodeId>);

    /// Adds a child syntax to this syntax.
    ///
    /// # Arguments
    ///
    /// - `child`: The child syntax’s ID to append.
    fn add_child(&mut self, child: NodeId);

    /// Returns the `NodeId` of the child at the given index, or an error if the index is out of bounds.
    ///
    /// # Arguments
    /// * `index` - The zero-based position of the child to retrieve.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if the syntax has fewer children than the requested index.
    ///
    /// # Example
    /// ```rust
    /// let child_id = syntax.try_child(0)?;
    /// let child_node = arena.try_node(child_id)?;
    /// ```
    ///
    /// # Panics
    /// This method does **not** panic. It returns a proper `Result`.
    fn try_child(&self, index: usize) -> Result<NodeId, AiplanError> {
        self.children()
            .get(index)
            .copied()
            .ok_or_else(|| AiplanError::InternalError(format!(
                "Expected child index {} in syntax kind {} but found only {} children",
                index,
                self.kind(),
                self.children().len()
            )))
    }

    /// Returns the `NodeId` of the child at the given index, or `None` if out of bounds.
    ///
    /// # Arguments
    /// * `index` - The zero-based position of the child to retrieve.
    ///
    /// # Example
    /// ```rust
    /// if let Some(child_id) = node.get_child_opt(0) {
    ///     let child_node = arena.try_node(child_id)?;
    /// }
    /// ```
    fn get_child(&self, index: usize) -> Option<NodeId> {
        self.children().get(index).copied()
    }

}

impl<N: ArenaNode> Arena<N>
where
    N::Kind: PartialEq + Copy,
{
    pub fn collect_nodes_of_kind(&self, kinds: &[N::Kind]) -> Vec<NodeId> {
        let mut nodes = Vec::new();
        for (id, node) in self.preorder_with_index() {
            if kinds.contains(&node.kind()) {
                nodes.push(id);
            }
        }
        nodes
    }
}
