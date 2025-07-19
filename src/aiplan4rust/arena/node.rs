use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use ordered_float::OrderedFloat;
use crate::aiplan4rust::arena::{Arena, NodeId, NodeContent};
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::symbol::SymbolRef;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, Ident, Optimization};
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
        self.parent().ok_or_else(|| AiplanError::new(format!(
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
            .ok_or_else(|| AiplanError::new(format!(
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

    /// Remaps identifiers inside the syntax’s content according to the given map.
    ///
    /// This is useful for operations like renaming or merging scopes.
    ///
    /// # Arguments
    ///
    /// - `map`: A `HashMap` mapping old identifiers to new identifiers.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>);

    /// Returns `true` if this syntax is a leaf (has no children).
    ///
    /// # Returns
    ///
    /// - `true` if the syntax has no children.
    /// - `false` otherwise.
    ///
    /// # Default
    ///
    /// Checks if `children().is_empty()`.
    fn is_leaf(&self) -> bool {
        self.children().is_empty()
    }

    /// Returns the number of direct children this syntax has.
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

    /// Returns `true` if this syntax is the root of the arena (has no parent).
    ///
    /// # Returns
    ///
    /// - `true` if `parent()` returns `None`.
    /// - `false` otherwise.
    fn is_root(&self) -> bool {
        self.parent().is_none()
    }


    // Delegation methods to the syntax’s content, allowing convenient extraction
    // of specific semantic types without manually matching on content.

    /// Returns the identifier if present in the syntax’s content.
    fn as_ident(&self) -> Option<Ident> {
        self.content().as_ident()
    }

    /// Returns the floating-point literal if present in the syntax’s content.
    fn as_float(&self) -> Option<OrderedFloat<f64>> {
        self.content().as_float()
    }

    /// Returns the binary comparison operator if present in the syntax’s content.
    fn as_binary_comp(&self) -> Option<BinaryComp> {
        self.content().as_binary_comp()
    }

    /// Returns the assignment operator if present in the syntax’s content.
    fn as_assign_op(&self) -> Option<AssignOp> {
        self.content().as_assign_op()
    }

    /// Returns the arithmetic operator if present in the syntax’s content.
    fn as_arithmetic_op(&self) -> Option<ArithmeticOp> {
        self.content().as_arithmetic_op()
    }

    /// Returns the optimization directive if present in the syntax’s content.
    fn as_optimization(&self) -> Option<Optimization> {
        self.content().as_optimization()
    }

    fn as_symbol_ref(&self) -> Result<Option<SymbolRef>, AiplanError>;

    // Try-extraction methods that return Result for better error handling.

    /// Attempts to extract an identifier from the syntax’s content.
    fn try_ident(&self) -> Result<Ident, AiplanError> {
        self.content().try_ident()
    }

    /// Attempts to extract a floating-point literal from the syntax’s content.
    fn try_float(&self) -> Result<OrderedFloat<f64>, AiplanError> {
        self.content().try_float()
    }

    /// Attempts to extract a binary comparison operator from the syntax’s content.
    fn try_binary_comp(&self) -> Result<BinaryComp, AiplanError> {
        self.content().try_binary_comp()
    }

    /// Attempts to extract an assignment operator from the syntax’s content.
    fn try_assign_op(&self) -> Result<AssignOp, AiplanError> {
        self.content().try_assign_op()
    }

    /// Attempts to extract an arithmetic operator from the syntax’s content.
    fn try_arithmetic_op(&self) -> Result<ArithmeticOp, AiplanError> {
        self.content().try_arithmetic_op()
    }

    /// Attempts to extract an optimization directive from the syntax’s content.
    fn try_optimization(&self) -> Result<Optimization, AiplanError> {
        self.content().try_optimization()
    }
    fn try_symbol_ref(&self) -> Result<SymbolRef, AiplanError> {
        self.as_symbol_ref()?.ok_or_else(|| AiplanError::new("Not a SymbolRef".to_string()))
    }

    /// Formats the syntax with access to the arena and an interner.
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
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, arena: &Arena<Self>, interner: &StringInterner) -> fmt::Result
    where Self: Sized;
    
    /// Converts the syntax to a string using `fmt_with`.
    ///
    /// # Arguments
    ///
    /// * `arena` - Reference to the arena to fetch other nodes if needed.
    /// * `interner` - Reference to the interner for resolving identifiers.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = syntax.to_string_with_interner(&arena, &interner);
    /// println!("{}", s);
    /// ```
    fn to_string_with_interner(&self, arena: &Arena<Self>, interner: &StringInterner) -> String
    where Self: Sized {
        struct DisplayWrapper<'a, T: ArenaNode> {
            node: &'a T,
            arena: &'a Arena<T>,
            interner: &'a StringInterner,
        }

        impl<'a, T: ArenaNode> fmt::Display for DisplayWrapper<'a, T> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result  {
                self.node.fmt_with_interner(f, self.arena, self.interner)
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

    /// Formats the syntax using a specific planning syntax style,
    /// applying the given indentation level.
    ///
    /// This method is similar to `fmt_with`, but formats the syntax
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
    fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        arena: &Arena<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result
    where
        Self: Sized;

    /// Formats the syntax using a specific planning syntax style with no indentation.
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
    fn fmt_syntax(
        &self,
        f: &mut Formatter<'_>,
        arena: &Arena<Self>,
        interner: &StringInterner,
    ) -> fmt::Result
    where
        Self: Sized,
    {
        self.fmt_syntax_with_indent(f, arena, interner, 0)
    }


    /// Converts the syntax to a string using a specific planning syntax format,
    /// applying the given indentation level.
    ///
    /// This method wraps the syntax in a temporary formatter to produce
    /// a syntax-oriented string representation using `fmt_planning`.
    ///
    /// # Arguments
    ///
    /// * `arena` - A reference to the arena containing the arena of nodes.
    /// * `interner` - A reference to the `StringInterner` used to resolve identifiers.
    /// * `indent` - The indentation level (number of indent units to apply).
    ///
    /// # Returns
    ///
    /// A `String` containing the formatted syntax representation of the syntax.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = syntax.to_planning_syntax_with_indent(&arena, &interner, 2);
    /// println!("{}", s);
    /// ```
    fn to_syntax_with_indent(
        &self,
        arena: &Arena<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> String
    where
        Self: Sized,
    {
        struct PlanningSyntaxDisplayWrapper<'a, T: ArenaNode> {
            node: &'a T,
            arena: &'a Arena<T>,
            interner: &'a StringInterner,
            indent: usize,
        }

        impl<'a, T: ArenaNode> fmt::Display
        for PlanningSyntaxDisplayWrapper<'a, T>
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.node.fmt_syntax_with_indent(f, self.arena, self.interner, self.indent)
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

    /// Converts the syntax to a string using the specific syntax formatting without indentation.
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
    /// let s = syntax.to_planning_syntax(&arena, &interner);
    /// println!("{}", s);
    /// ```
    fn to_syntax_string(
        &self,
        arena: &Arena<Self>,
        interner: &StringInterner,
    ) -> String
    where
        Self: Sized,
    {
        self.to_syntax_with_indent(arena, interner, 0)
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
