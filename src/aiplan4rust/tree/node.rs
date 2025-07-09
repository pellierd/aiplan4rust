use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use ordered_float::OrderedFloat;
use crate::aiplan4rust::tree::{TreeArena, NodeId, NodeContent};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::symbol::SymbolRef;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, Ident, Optimization};
use crate::aiplan4rust::syntax::PlanningSyntaxDisplay;

/// A generic trait representing a node in a tree stored within an `Arena`.
///
/// This trait defines the minimal interface that any node type must implement to
/// be used in a tree structure managed by a `TreeArena`. It provides methods to
/// navigate parent-child relationships, modify the tree, work with node content,
/// and extract semantic information like identifiers and symbols.
///
/// # Trait Type Parameters
///
/// - `Kind`: The type representing the kind/category of the node.
/// - `Content`: The type of the node’s semantic content, which must implement [`NodeContent`].
///
/// # Core Responsibilities
///
/// - Access and modify the node's kind and content.
/// - Navigate the tree structure: get parent and children, add children.
/// - Remap identifiers within the node's content using a mapping table.
/// - Query node properties such as leaf/root status and arity (number of children).
/// - Attempt to extract a symbol reference from the node within a tree arena.
///
/// # Example
///
/// ```rust
/// use std::collections::HashMap;
/// use crate::aiplan4rust::tree::{TreeNode, NodeId};
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
/// - [`TreeArena`] for managing trees of nodes implementing this trait.
/// - [`NodeContent`] for content types that hold semantic node data.
/// - [`ParserInternalError`] for error handling during parsing or resolution.
/// - [`SymbolRef`] for referencing symbols resolved from nodes.
///
pub trait TreeNode {
    /// The type used to represent the node's kind.
    type Kind: std::fmt::Display;

    /// The type used to represent the semantic content of the node.
    type Content: NodeContent;

    /// Returns the kind of the node.
    fn kind(&self) -> Self::Kind;

    /// Sets the kind of the node.
    fn set_kind(&mut self, kind: Self::Kind);

    /// Returns a reference to the node's semantic content.
    fn content(&self) -> &Self::Content;

    /// Returns a mutable reference to the node's semantic content.
    fn content_mut(&mut self) -> &mut Self::Content;

    /// Returns `true` if the node has no meaningful content.
    ///
    /// By default, this delegates to `content().is_none()`.
    fn has_content(&self) -> bool {
        self.content().is_none()
    }

    /// Returns the ID of the node’s parent if it exists.
    ///
    /// # Returns
    ///
    /// - `Some(NodeId)` if the node has a parent.
    /// - `None` if this node is the root of the tree.
    fn parent(&self) -> Option<NodeId>;

    /// Returns the `NodeId` of the parent of this node, or an error if there is no parent.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if this node has no parent.
    ///
    /// # Example
    /// ```rust
    /// let parent_id = node.try_parent()?;
    /// let parent_node = arena.try_node(parent_id)?;
    /// ```
    ///
    /// # Panics
    /// This method does **not** panic. It returns a proper `Result`.
    fn try_parent(&self) -> Result<NodeId, ParserInternalError> {
        self.parent().ok_or_else(|| ParserInternalError::new(format!(
            "Expected parent for node kind {} but found none",
            self.kind()
        )))
    }

    /// Sets the parent of this node.
    ///
    /// # Arguments
    ///
    /// - `parent`: The parent node's ID, or `None` to unset and mark this node as root.
    fn set_parent(&mut self, parent: Option<NodeId>);

    /// Returns a slice of IDs representing this node’s immediate children.
    ///
    /// # Returns
    ///
    /// A slice of `NodeId` elements corresponding to the children.
    fn children(&self) -> &[NodeId];

    /// Sets the immediate children of this node to the given list of node IDs.
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
    /// node.set_children(vec![child1, child2]);
    /// ```
    fn set_children(&mut self, children: Vec<NodeId>);

    /// Adds a child node to this node.
    ///
    /// # Arguments
    ///
    /// - `child`: The child node’s ID to append.
    fn add_child(&mut self, child: NodeId);

    /// Returns the `NodeId` of the child at the given index, or an error if the index is out of bounds.
    ///
    /// # Arguments
    /// * `index` - The zero-based position of the child to retrieve.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if the node has fewer children than the requested index.
    ///
    /// # Example
    /// ```rust
    /// let child_id = node.try_child(0)?;
    /// let child_node = arena.try_node(child_id)?;
    /// ```
    ///
    /// # Panics
    /// This method does **not** panic. It returns a proper `Result`.
    fn try_child(&self, index: usize) -> Result<NodeId, ParserInternalError> {
        self.children()
            .get(index)
            .copied()
            .ok_or_else(|| ParserInternalError::new(format!(
                "Expected child index {} in node kind {} but found only {} children",
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

    /// Remaps identifiers inside the node’s content according to the given map.
    ///
    /// This is useful for operations like renaming or merging scopes.
    ///
    /// # Arguments
    ///
    /// - `map`: A `HashMap` mapping old identifiers to new identifiers.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>);

    /// Returns `true` if this node is a leaf (has no children).
    ///
    /// # Returns
    ///
    /// - `true` if the node has no children.
    /// - `false` otherwise.
    ///
    /// # Default
    ///
    /// Checks if `children().is_empty()`.
    fn is_leaf(&self) -> bool {
        self.children().is_empty()
    }

    /// Returns the number of direct children this node has.
    ///
    /// # Returns
    ///
    /// The count of immediate child nodes.
    ///
    /// # Default
    ///
    /// Returns `children().len()`.
    fn arity(&self) -> usize {
        self.children().len()
    }

    /// Returns `true` if this node is the root of the tree (has no parent).
    ///
    /// # Returns
    ///
    /// - `true` if `parent()` returns `None`.
    /// - `false` otherwise.
    fn is_root(&self) -> bool {
        self.parent().is_none()
    }


    // Delegation methods to the node’s content, allowing convenient extraction
    // of specific semantic types without manually matching on content.

    /// Returns the identifier if present in the node’s content.
    fn as_ident(&self) -> Option<Ident> {
        self.content().as_ident()
    }

    /// Returns the floating-point literal if present in the node’s content.
    fn as_float(&self) -> Option<OrderedFloat<f64>> {
        self.content().as_float()
    }

    /// Returns the binary comparison operator if present in the node’s content.
    fn as_binary_comp(&self) -> Option<BinaryComp> {
        self.content().as_binary_comp()
    }

    /// Returns the assignment operator if present in the node’s content.
    fn as_assign_op(&self) -> Option<AssignOp> {
        self.content().as_assign_op()
    }

    /// Returns the arithmetic operator if present in the node’s content.
    fn as_arithmetic_op(&self) -> Option<ArithmeticOp> {
        self.content().as_arithmetic_op()
    }

    /// Returns the optimization directive if present in the node’s content.
    fn as_optimization(&self) -> Option<Optimization> {
        self.content().as_optimization()
    }

    fn as_symbol_ref(&self) -> Result<Option<SymbolRef>, ParserInternalError>;

    // Try-extraction methods that return Result for better error handling.

    /// Attempts to extract an identifier from the node’s content.
    fn try_ident(&self) -> Result<Ident, ParserInternalError> {
        self.content().try_ident()
    }

    /// Attempts to extract a floating-point literal from the node’s content.
    fn try_float(&self) -> Result<OrderedFloat<f64>, ParserInternalError> {
        self.content().try_float()
    }

    /// Attempts to extract a binary comparison operator from the node’s content.
    fn try_binary_comp(&self) -> Result<BinaryComp, ParserInternalError> {
        self.content().try_binary_comp()
    }

    /// Attempts to extract an assignment operator from the node’s content.
    fn try_assign_op(&self) -> Result<AssignOp, ParserInternalError> {
        self.content().try_assign_op()
    }

    /// Attempts to extract an arithmetic operator from the node’s content.
    fn try_arithmetic_op(&self) -> Result<ArithmeticOp, ParserInternalError> {
        self.content().try_arithmetic_op()
    }

    /// Attempts to extract an optimization directive from the node’s content.
    fn try_optimization(&self) -> Result<Optimization, ParserInternalError> {
        self.content().try_optimization()
    }
    fn try_symbol_ref(&self) -> Result<SymbolRef, ParserInternalError> {
        self.as_symbol_ref()?.ok_or_else(|| ParserInternalError::new("Not a SymbolRef".to_string()))
    }

    /// Formats the node with access to the arena and an interner.
    ///
    /// This method allows accessing other nodes in the arena,
    /// useful for displaying children or related information.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `arena` - Reference to the arena containing all nodes.
    /// * `interner` - Reference to the interner for resolving identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails.
    fn fmt_with(&self, f: &mut Formatter<'_>, arena: &TreeArena<Self>, interner: &StringInterner) -> fmt::Result
    where Self: Sized;
    
    /// Converts the node to a string using `fmt_with`.
    ///
    /// # Arguments
    ///
    /// * `arena` - Reference to the arena to fetch other nodes if needed.
    /// * `interner` - Reference to the interner for resolving identifiers.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = node.to_string_with_interner(&arena, &interner);
    /// println!("{}", s);
    /// ```
    fn to_string_with_interner(&self, arena: &TreeArena<Self>, interner: &StringInterner) -> String
    where Self: Sized {
        struct DisplayWrapper<'a, T: TreeNode> {
            node: &'a T,
            arena: &'a TreeArena<T>,
            interner: &'a StringInterner,
        }

        impl<'a, T: TreeNode> fmt::Display for DisplayWrapper<'a, T> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result  {
                self.node.fmt_with(f, self.arena, self.interner)
            }
        }

        format!("{}", DisplayWrapper { node: self, arena, interner })
    }


    /// Number of characters per indentation level.
    fn indent_width() -> usize {
        2
    }

    /// Character used for indentation.
    fn indent_char() -> char {
        ' '
    }

    /// Builds the indentation string for a given level.
    fn make_indent(level: usize) -> String {
        let total = level * Self::indent_width();
        std::iter::repeat(Self::indent_char())
            .take(total)
            .collect()
    }

    /// Formats the node using a specific planning syntax style,
    /// applying the given indentation level.
    ///
    /// This method is similar to `fmt_with`, but formats the node
    /// according to a custom grammar or syntax conventions (for example,
    /// PDDL-like syntax). Implementors should respect the provided
    /// `indent` level when producing their output.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `arena` - A reference to the arena containing all nodes.
    /// * `interner` - A reference to the `StringInterner` for resolving identifiers.
    /// * `indent` - The indentation level (number of indent units).
    ///
    /// # Errors
    ///
    /// Returns a [`fmt::Error`] if writing to the formatter fails.
    fn fmt_planning_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        arena: &TreeArena<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result
    where
        Self: Sized;

    /// Formats the node using a specific planning syntax style with no indentation.
    ///
    /// This is a convenience method that simply calls
    /// [`fmt_planning_syntax_with_indent`] with an indent level of 0.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `arena` - A reference to the arena containing all nodes.
    /// * `interner` - A reference to the `StringInterner` for resolving identifiers.
    ///
    /// # Errors
    ///
    /// Returns a [`fmt::Error`] if writing to the formatter fails.
    fn fmt_planning_syntax(
        &self,
        f: &mut Formatter<'_>,
        arena: &TreeArena<Self>,
        interner: &StringInterner,
    ) -> fmt::Result
    where
        Self: Sized,
    {
        self.fmt_planning_syntax_with_indent(f, arena, interner, 0)
    }


    /// Converts the node to a string using a specific planning syntax format,
    /// applying the given indentation level.
    ///
    /// This method wraps the node in a temporary formatter to produce
    /// a syntax-oriented string representation using `fmt_planning`.
    ///
    /// # Arguments
    ///
    /// * `arena` - A reference to the arena containing the tree of nodes.
    /// * `interner` - A reference to the `StringInterner` used to resolve identifiers.
    /// * `indent` - The indentation level (number of indent units to apply).
    ///
    /// # Returns
    ///
    /// A `String` containing the formatted syntax representation of the node.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = node.to_planning_syntax_with_indent(&arena, &interner, 2);
    /// println!("{}", s);
    /// ```
    fn to_planning_syntax_with_indent(
        &self,
        arena: &TreeArena<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> String
    where
        Self: Sized,
    {
        struct PlanningSyntaxDisplayWrapper<'a, T: TreeNode> {
            node: &'a T,
            arena: &'a TreeArena<T>,
            interner: &'a StringInterner,
            indent: usize,
        }

        impl<'a, T: TreeNode> fmt::Display
        for PlanningSyntaxDisplayWrapper<'a, T>
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.node.fmt_planning_syntax_with_indent(f, self.arena, self.interner, self.indent)
            }
        }

        format!(
            "{}",
            PlanningSyntaxDisplayWrapper {
                node: self,
                arena,
                interner,
                indent
            }
        )
    }


    /// Converts the node to a string using the specific syntax formatting without indentation.
    ///
    /// This simply calls `to_planning_syntax_with_indent` with an indent level of 0.
    ///
    /// # Arguments
    ///
    /// * `arena` - Reference to the arena containing the nodes.
    /// * `interner` - Reference to the interner for resolving identifiers.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = node.to_planning_syntax(&arena, &interner);
    /// println!("{}", s);
    /// ```
    fn to_planning_syntax(
        &self,
        arena: &TreeArena<Self>,
        interner: &StringInterner,
    ) -> String
    where
        Self: Sized,
    {
        self.to_planning_syntax_with_indent(arena, interner, 0)
    }
}
