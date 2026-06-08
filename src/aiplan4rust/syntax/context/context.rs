//! Parsing context module used by the LALRPOP-generated parser.
//!
//! This module provides the [`ParseContext`] structure, which acts as the central
//! context object during parsing. It is responsible for managing:
//!
//! - A [`Tree`] of arena-allocated AST nodes,
//! - A [`SymbolInterner`] for deduplicating string identifiers,
//! - A list of recoverable lexical or syntactic errors.
//!
//! ## Components
//! - [`ParseContext`]: Owns and coordinates syntax tree, interner, and error list.
//! - [`Tree`]: Arena-based tree used to allocate and structure [`AstNode`]s.
//! - [`SymbolInterner`]: Deduplicates and manages unique string identifiers.
//!
//! ## Responsibilities
//! `ParseContext` is passed throughout the parsing pipeline and serves to:
//!
//! - Allocate and link AST nodes in the [`Tree`],
//! - Intern all strings used in identifiers (`Ident`),
//! - Collect recoverable errors (`ErrorRecovery`) produced during parsing.
//!
//! ## Errors
//! Most fallible methods return [`ParseContextError`] which may wrap:
//! - [`ArenaError`] (via `SyntaxTree`),
//! - [`SyntaxTreeError`] (invalid operations on syntax nodes).
//!
//! ## Example
//! ```
//! let ctx = ParseContext::new();
//! // Used inside LALRPOP parser actions...
//! if ctx.has_errors() {
//!     for err in ctx.borrow_errors().iter() {
//!         eprintln!("Parsing error: {:?}", err);
//!     }
//! }
//! ```
//!
//! This module is intended for internal use by the parser.

use lalrpop_util::{ErrorRecovery, ParseError};
use std::cell::RefCell;

use crate::aiplan4rust::core::interner::SymbolInterner;
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::syntax::ast::tree::{NodeId, Tree};
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind, AstNode};
use crate::aiplan4rust::syntax::context::error::ParseContextError;
use crate::aiplan4rust::syntax::lexer::Token;
use crate::aiplan4rust::syntax::CustomParseError;
use crate::aiplan4rust::syntax::Span;

/// Parsing context used throughout the LALRPOP parsing process.
///
/// The `ParseContext` owns and manages:
/// - A [`Tree`] of AST nodes, used to construct the program structure.
/// - A [`SymbolInterner`] to deduplicate and reference strings used in identifiers.
/// - A list of recoverable [`CustomParseError`]s and parse errors.
///
/// It is passed into the parser and incrementally filled during parsing.
/// Afterward, it provides access to the root node, interned strings,
/// and all collected errors.
pub struct ParseContext {
    interner: RefCell<SymbolInterner>,
    syntax_tree: RefCell<Tree<AstNode>>,
    errors: RefCell<Vec<ErrorRecovery<usize, Token, CustomParseError>>>,
}

impl ParseContext {
    /// Creates a new `ParseContext` and attempts to initialize the PDDL built-in nodes.
    ///
    /// This constructor ensures that the AST arena is pre-populated with reserved
    /// language symbols (NodeIds 0 to 5) before any parsing occurs.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseContextError`] if the initial allocation of built-in nodes fails.
    /// This usually indicates an issue with the AST tree or arena allocation.
    pub fn new() -> Result<Self, ParseContextError> {
        let ctx = Self {
            interner: RefCell::new(SymbolInterner::new()),
            syntax_tree: RefCell::new(Tree::new()),
            errors: RefCell::new(Vec::new()),
        };

        // Initialize built-in nodes. If this fails, we propagate the error
        // instead of panicking, allowing for graceful error handling.
        ctx.init_builtins()?;

        Ok(ctx)
    }

    /// Initializes the language's intrinsic (built-in) nodes within the AST arena.
    ///
    /// This method populates the first slots of the [`Tree`] with reserved PDDL symbols.
    /// The order of calls is **critical** as it ensures a 1:1 mapping between the [`NodeId`]
    /// in the AST and the [`SymbolId`] in the interner:
    ///
    /// | NodeId | Symbol | AST Kind | Usage |
    /// | :--- | :--- | :--- | :--- |
    /// | `0` | `object` | `PrimitiveType` | Root type of the hierarchy. |
    /// | `1` | `number` | `PrimitiveType` | Numerical type for fluents. |
    /// | `2` | `?duration` | `Variable` | Special variable for durative actions. |
    /// | `3` | `total-time` | `FunctionSymbol` | Elapsed time metric. |
    /// | `4` | `total-cost` | `FunctionSymbol` | Action cost metric. |
    /// | `5` | `#t` | `Variable` | Continuous time variable (PDDL+). |
    ///
    /// # Technical Details
    /// - **Virtual Spans**: All these nodes use `usize::MAX` for their start and end
    ///   coordinates. This allows diagnostic engines to identify them as system-defined
    ///   nodes with no physical presence in the source code.
    /// - **Immutability**: This function must be called exactly once during the
    ///   [`ParseContext`] creation to ensure indexing integrity.
    ///
    /// # Errors
    /// Returns a [`ParseContextError`] if the allocation in the arena fails or if the
    /// tree structure is corrupted during initialization.
    pub fn init_builtins(&self) -> Result<(), ParseContextError> {
        let virtual_pos = usize::MAX;

        // Explicit definition of the built-ins structure.
        // The index in this array determines the final NodeId.
        let builtins = [
            (AstKind::PrimitiveType, SymbolInterner::OBJECT_SYMBOL_ID), // ID 0
            (AstKind::PrimitiveType, SymbolInterner::NUMBER_SYMBOL_ID), // ID 1
            (
                AstKind::Variable,
                SymbolInterner::DURATION_VARIABLE_SYMBOL_ID,
            ), // ID 2
            (
                AstKind::FunctionSymbol,
                SymbolInterner::TOTAL_TIME_SYMBOL_ID,
            ), // ID 3
            (
                AstKind::FunctionSymbol,
                SymbolInterner::TOTAL_COST_SYMBOL_ID,
            ), // ID 4
            (
                AstKind::Variable,
                SymbolInterner::CONTINUOUS_VARIABLE_SYMBOL_ID,
            ), // ID 5
        ];

        for (kind, symbol_id) in builtins {
            self.alloc_node(
                kind,
                AstContent::Ident(symbol_id),
                vec![],
                virtual_pos,
                virtual_pos,
            )?;
        }

        Ok(())
    }

    /// Provides read-only access to the underlying syntax tree arena.
    ///
    /// # Returns
    ///
    /// A shared reference to the syntax tree.
    ///
    /// # Panics
    ///
    /// Panics if the internal borrow rules are violated.
    pub fn borrow_syntax_tree(&self) -> std::cell::Ref<'_, Tree<AstNode>> {
        self.syntax_tree.borrow()
    }

    /// Provides mutable access to the underlying syntax tree arena.
    ///
    /// # Returns
    ///
    /// A mutable reference to the syntax tree.
    ///
    /// # Panics
    ///
    /// Panics if there is an existing active borrow (mutable or immutable).
    pub fn borrow_syntax_tree_mut(&self) -> std::cell::RefMut<'_, Tree<AstNode>> {
        self.syntax_tree.borrow_mut()
    }

    /// Takes ownership of the syntax tree and replaces it with an empty one.
    ///
    /// # Returns
    ///
    /// The previously held `SyntaxTree`.
    ///
    /// # Notes
    ///
    /// After this operation, the internal syntax tree is reset.
    pub fn take_syntax_tree(&self) -> Tree<AstNode> {
        std::mem::take(&mut *self.syntax_tree.borrow_mut())
    }

    /// Sets the root node ID in the syntax tree.
    ///
    /// # Arguments
    ///
    /// * `root_id` - The node ID to set as the root.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the root was successfully set.
    ///
    /// # Errors
    ///
    /// Returns [`ParseContextError::SyntaxTree`] if the ID does not refer to a valid node.
    pub fn set_root_id(&self, root_id: NodeId) -> Result<(), ParseContextError> {
        self.syntax_tree.borrow_mut().set_root_id(root_id)?;
        Ok(())
    }

    /// Returns the ID of the root syntax node, if set.
    ///
    /// # Returns
    ///
    /// An [`Option<NodeId>`] representing the root node ID if it exists.
    pub fn root_id(&self) -> Option<NodeId> {
        self.syntax_tree.borrow().root_id()
    }

    /// Allocates a new AST node into the syntax tree arena.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of syntax node to allocate.
    /// * `content` - The semantic content of the node.
    /// * `children` - A list of child node IDs.
    /// * `start` - The starting byte offset in the source code.
    /// * `end` - The ending byte offset in the source code.
    ///
    /// # Returns
    ///
    /// `Ok(NodeId)` if the node was successfully allocated.
    ///
    /// # Errors
    ///
    /// Returns [`ParseContextError::Arena`] if memory allocation or child updates fail.
    /// Returns [`ParseContextError::SyntaxTree`] if node access fails internally.
    pub fn alloc_node(
        &self,
        kind: AstKind,
        content: AstContent,
        children: Vec<NodeId>,
        start: usize,
        end: usize,
    ) -> Result<NodeId, ParseContextError> {
        let span = Span::new(start, end);
        let node = AstNode::new(kind, content, children, span, None);
        let mut syntax_tree = self.syntax_tree.borrow_mut();
        let node_id = syntax_tree.alloc(node);

        let children_ids = {
            let stored = syntax_tree.try_node(node_id)?;
            stored.children().to_vec()
        };

        for child in &children_ids {
            if let Some(child_node) = syntax_tree.get_node_mut(*child) {
                child_node.set_parent(Some(node_id));
            }
        }

        Ok(node_id)
    }

    /// Merges the children of `next` into `typed_list`, updating parent links.
    ///
    /// # Errors
    /// Returns [`ParseContextError`] if nodes cannot be accessed.
    pub fn merge_typed_list(
        &mut self,
        typed_list: NodeId,
        next: NodeId,
    ) -> Result<NodeId, ParseContextError> {
        let mut syntax_tree = self.borrow_syntax_tree_mut();

        let next_children = {
            let next_node = syntax_tree.try_node_mut(next)?;
            std::mem::take(next_node.children_mut())
        };

        if !next_children.is_empty() {
            for child in &next_children {
                let child_node = syntax_tree.try_node_mut(*child)?;
                child_node.set_parent(Some(typed_list));
            }

            let typed_list_node = syntax_tree.try_node_mut(typed_list)?;
            typed_list_node.children_mut().extend(next_children);
        }

        Ok(typed_list)
    }

    // String interning

    /// Interns a string and returns its unique identifier.
    ///
    /// If the string has already been interned, this method returns the existing identifier,
    /// ensuring that each unique string has a single associated `Ident`.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to intern.
    ///
    /// # Returns
    ///
    /// An `Ident` representing the unique identifier for the interned string.
    ///
    /// # Example
    ///
    /// ```
    /// let id1 = ctx.intern("var".to_string());
    /// let id2 = ctx.intern("var".to_string());
    /// assert_eq!(id1, id2);
    /// ```
    pub fn intern(&self, s: String) -> SymbolId {
        self.interner.borrow_mut().intern_symbol(s)
    }

    /// Provides shared access to the underlying string interner.
    ///
    /// # Returns
    ///
    /// A shared reference (`Ref`) to the `StringInterner`, allowing read-only operations.
    ///
    /// # Errors
    ///
    /// This function may panic if the `RefCell` is currently mutably borrowed.
    pub fn borrow_interner(&self) -> std::cell::Ref<'_, SymbolInterner> {
        self.interner.borrow()
    }

    /// Provides mutable access to the underlying string interner.
    ///
    /// # Returns
    ///
    /// A mutable reference (`RefMut`) to the `StringInterner`, allowing modification.
    ///
    /// # Errors
    ///
    /// This function may panic if the `RefCell` is currently borrowed.
    pub fn borrow_interner_mut(&self) -> std::cell::RefMut<'_, SymbolInterner> {
        self.interner.borrow_mut()
    }

    /// Takes ownership of the current `StringInterner`, replacing it with a new, empty instance.
    ///
    /// # Returns
    ///
    /// The current `StringInterner` instance, consuming the internal one.
    ///
    /// # Notes
    ///
    /// After this operation, the internal interner is reset to an empty state.
    pub fn take_interner(&self) -> SymbolInterner {
        std::mem::take(&mut *self.interner.borrow_mut())
    }

    // Error collection

    /// Returns all accumulated recoverable errors (lexical or syntactic).
    ///
    /// # Returns
    ///
    /// A shared reference (`Ref`) to a vector containing all `ErrorRecovery` instances
    /// recorded so far. These represent recoverable errors encountered during parsing,
    /// including lexical and syntactic errors.
    ///
    /// # Errors
    ///
    /// This function does not return errors directly but may panic if the internal
    /// borrow rules of `RefCell` are violated (which should not happen under normal use).
    pub fn borrow_errors(
        &self,
    ) -> std::cell::Ref<'_, Vec<ErrorRecovery<usize, Token, CustomParseError>>> {
        self.errors.borrow()
    }

    /// Provides mutable access to the accumulated recoverable errors.
    ///
    /// # Returns
    ///
    /// A mutable reference (`RefMut`) to the vector of `ErrorRecovery` instances,
    /// allowing modification of the error list.
    ///
    /// # Errors
    ///
    /// This function may panic if a mutable borrow conflict occurs on the internal `RefCell`.
    pub fn borrow_errors_mut(
        &self,
    ) -> std::cell::RefMut<'_, Vec<ErrorRecovery<usize, Token, CustomParseError>>> {
        self.errors.borrow_mut()
    }

    /// Checks if any recoverable errors have been recorded.
    ///
    /// # Returns
    ///
    /// `true` if one or more errors exist in the accumulated error list, otherwise `false`.
    pub fn has_errors(&self) -> bool {
        !self.errors.borrow().is_empty()
    }

    /// Clears all recorded errors and returns them.
    ///
    /// # Returns
    ///
    /// A vector containing all previously recorded `ErrorRecovery` instances.
    ///
    /// # Notes
    ///
    /// After calling this method, the internal error list will be empty.
    pub fn take_errors(&self) -> Vec<ErrorRecovery<usize, Token, CustomParseError>> {
        std::mem::take(&mut *self.errors.borrow_mut())
    }

    /// Adds a new recoverable error to the accumulated list.
    ///
    /// # Arguments
    ///
    /// * `error` - An `ErrorRecovery` instance representing a recoverable lexical or syntactic error.
    ///
    /// # Behavior
    ///
    /// The provided error is appended to the internal error list.
    pub fn push_error(&self, error: ErrorRecovery<usize, Token, CustomParseError>) {
        self.errors.borrow_mut().push(error);
    }

    /// Adds a new custom recoverable parse error to the accumulated list.
    ///
    /// # Arguments
    ///
    /// * `error` - A `CustomParseError` instance representing a recoverable parser-specific error.
    ///
    /// # Behavior
    ///
    /// This function wraps the provided `CustomParseError` in a `ParseError::User` variant,
    /// then pushes it into the internal recoverable error list. Parsing can continue
    /// despite this error being recorded.
    pub fn push_custom_error(&self, error: CustomParseError) {
        self.push_error(ErrorRecovery {
            dropped_tokens: vec![],
            error: ParseError::User { error },
        });
    }
}
