//! Parsing context module used by the LALRPOP-generated parser.
//!
//! This module provides the [`ParseContext`] structure, which acts as the central
//! context object during parsing. It is responsible for managing:
//!
//! - A [`SyntaxTree`] of arena-allocated AST nodes,
//! - A [`StringInterner`] for deduplicating string identifiers,
//! - A list of recoverable lexical or syntactic errors.
//!
//! ## Components
//! - [`ParseContext`]: Owns and coordinates syntax tree, interner, and error list.
//! - [`SyntaxTree`]: Arena-based tree used to allocate and structure [`AstNode`]s.
//! - [`StringInterner`]: Deduplicates and manages unique string identifiers.
//!
//! ## Responsibilities
//! `ParseContext` is passed throughout the parsing pipeline and serves to:
//!
//! - Allocate and link AST nodes in the [`SyntaxTree`],
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

use std::cell::RefCell;
use lalrpop_util::ErrorRecovery;

use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind, AstNode};
use crate::aiplan4rust::syntax::context::error::ParseContextError;
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxTree};
use crate::aiplan4rust::syntax::Span;

/// Parsing context used throughout the LALRPOP parsing process.
///
/// The `ParseContext` owns and manages:
/// - A [`SyntaxTree`] of AST nodes, used to construct the program structure.
/// - A [`StringInterner`] to deduplicate and reference strings used in identifiers.
/// - A list of recoverable [`LexicalError`]s and parse errors.
///
/// It is passed into the parser and incrementally filled during parsing.
/// Afterward, it provides access to the root node, interned strings,
/// and all collected errors.
pub struct ParseContext {
    interner: RefCell<StringInterner>,
    syntax_tree: RefCell<SyntaxTree<AstNode>>,
    errors: RefCell<Vec<ErrorRecovery<usize, Token, LexicalError>>>,
}

impl ParseContext {
    /// Creates a new, empty `ParseContext`.
    ///
    /// Initializes an empty `StringInterner`, an empty `SyntaxTree`, and an empty error buffer.
    ///
    /// # Returns
    ///
    /// A fresh `ParseContext` instance.
    ///
    /// # Example
    ///
    /// ```
    /// let ctx = ParseContext::new();
    /// assert!(ctx.root_id().is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            interner: RefCell::new(StringInterner::new()),
            syntax_tree: RefCell::new(SyntaxTree::empty()),
            errors: RefCell::new(Vec::new()),
        }
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
    pub fn borrow_syntax_tree(&self) -> std::cell::Ref<'_, SyntaxTree<AstNode>> {
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
    pub fn borrow_syntax_tree_mut(&self) -> std::cell::RefMut<'_, SyntaxTree<AstNode>> {
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
    pub fn take_syntax_tree(&self) -> SyntaxTree<AstNode> {
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
    pub fn intern(&self, s: String) -> Ident {
        self.interner.borrow_mut().intern_ident(s)
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
    pub fn borrow_interner(&self) -> std::cell::Ref<'_, StringInterner> {
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
    pub fn borrow_interner_mut(&self) -> std::cell::RefMut<'_, StringInterner> {
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
    pub fn take_interner(&self) -> StringInterner {
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
    pub fn borrow_errors(&self) -> std::cell::Ref<'_, Vec<ErrorRecovery<usize, Token, LexicalError>>> {
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
    pub fn borrow_errors_mut(&self) -> std::cell::RefMut<'_, Vec<ErrorRecovery<usize, Token, LexicalError>>> {
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
    pub fn take_errors(&self) -> Vec<ErrorRecovery<usize, Token, LexicalError>> {
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
    pub fn push_error(&self, error: ErrorRecovery<usize, Token, LexicalError>) {
        self.errors.borrow_mut().push(error);
    }

}
