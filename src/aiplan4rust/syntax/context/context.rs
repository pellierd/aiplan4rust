//! Module providing parsing context used by the LALRPOP-generated parser.
//!
//! The `ParseContext` struct acts as the central structure during parsing,
//! containing mutable access to:
//! - the [`Arena`] storing the AST nodes,
//! - the [`StringInterner`] for deduplicating strings,
//! - the list of lexical or syntactic errors collected during parsing.
//!
//! This context is passed around throughout the parsing process and is
//! responsible for managing syntax allocation, error recording, and interning
//! strings.
//!
//! # Components
//! - [`ParseContext`] manages arena-allocated AST nodes and error collection.
//! - [`Arena`] is a generic arena allocator for tree structures.
//! - [`StringInterner`] reduces memory usage by deduplicating strings.
//!
//! # Errors
//! Most methods return [`ParseContextError`] if something goes wrong,
//! which typically wraps lower-level [`ArenaError`] or [`AstError`].
//!
//! This module is designed to be used internally by the parser.

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
/// - An arena of AST nodes, which forms the tree structure of the parsed program.
/// - A string interner to efficiently handle and deduplicate identifiers.
/// - A list of recoverable lexer/parser errors.
///
/// This structure is passed into the parser and progressively filled during
/// parsing. After parsing, it provides access to the AST root, interned strings,
/// and collected errors.
pub struct ParseContext {
    interner: RefCell<StringInterner>,
    arena: RefCell<SyntaxTree<AstNode>>,
    errors: RefCell<Vec<ErrorRecovery<usize, Token, LexicalError>>>,
}

impl ParseContext {
    /// Creates a new, empty `ParseContext`.
    ///
    /// # Example
    /// ```
    /// let ctx = ParseContext::new();
    /// assert!(ctx.root_id().is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            interner: RefCell::new(StringInterner::new()),
            arena: RefCell::new(SyntaxTree::empty()),
            errors: RefCell::new(Vec::new()),
        }
    }

    /// Provides read-only access to the arena.
    pub fn borrow_arena(&self) -> std::cell::Ref<'_, SyntaxTree<AstNode>> {
        self.arena.borrow()
    }

    /// Provides mutable access to the arena.
    ///
    /// # Panics
    /// Panics if another borrow (mutable or immutable) is still active.
    pub fn borrow_arena_mut(&self) -> std::cell::RefMut<'_, SyntaxTree<AstNode>> {
        self.arena.borrow_mut()
    }

    /// Takes ownership of the arena and replaces it with a new empty arena.
    pub fn take_arena(&self) -> SyntaxTree<AstNode> {
        std::mem::take(&mut *self.arena.borrow_mut())
    }

    /// Sets the root syntax ID in the arena.
    ///
    /// # Errors
    /// Returns [`ParseContextError`] if the syntax ID does not exist.
    pub fn set_root_id(&self, root_id: NodeId) -> Result<(), ParseContextError> {
        self.arena.borrow_mut().set_root_id(root_id)?;
        Ok(())
    }

    /// Returns the ID of the root syntax, if any.
    pub fn root_id(&self) -> Option<NodeId> {
        self.arena.borrow().root_id()
    }

    /// Allocates a new AST node in the arena.
    ///
    /// # Arguments
    /// - `kind` – The node kind.
    /// - `content` – The AST content.
    /// - `children` – Children nodes.
    /// - `start` / `end` – Span positions.
    ///
    /// # Returns
    /// A `NodeId` if successful.
    ///
    /// # Errors
    /// Returns [`ParseContextError`] if allocation or child update fails.
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
        let mut arena = self.arena.borrow_mut();
        let node_id = arena.alloc(node);

        let children_ids = {
            let stored = arena.try_node(node_id)?;
            stored.children().to_vec()
        };

        for child in &children_ids {
            if let Some(child_node) = arena.get_node_mut(*child) {
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
        let mut arena = self.borrow_arena_mut();

        let next_children = {
            let next_node = arena.try_node_mut(next)?;
            std::mem::take(next_node.children_mut())
        };

        if !next_children.is_empty() {
            for child in &next_children {
                let child_node = arena.try_node_mut(*child)?;
                child_node.set_parent(Some(typed_list));
            }

            let typed_list_node = arena.try_node_mut(typed_list)?;
            typed_list_node.children_mut().extend(next_children);
        }

        Ok(typed_list)
    }

    // String interning

    /// Interns a string and returns its unique identifier.
    ///
    /// If the string already exists, reuses its identifier.
    ///
    /// # Example
    /// ```
    /// let id1 = ctx.intern("var".to_string());
    /// let id2 = ctx.intern("var".to_string());
    /// assert_eq!(id1, id2);
    /// ```
    pub fn intern(&self, s: String) -> Ident {
        self.interner.borrow_mut().intern(s)
    }

    pub fn borrow_interner(&self) -> std::cell::Ref<'_, StringInterner> {
        self.interner.borrow()
    }

    pub fn borrow_interner_mut(&self) -> std::cell::RefMut<'_, StringInterner> {
        self.interner.borrow_mut()
    }

    pub fn take_interner(&self) -> StringInterner {
        std::mem::take(&mut *self.interner.borrow_mut())
    }

    // Error collection

    /// Returns all accumulated recoverable errors (lexical/syntactic).
    pub fn borrow_errors(&self) -> std::cell::Ref<'_, Vec<ErrorRecovery<usize, Token, LexicalError>>> {
        self.errors.borrow()
    }

    /// Mutable access to accumulated errors.
    pub fn borrow_errors_mut(&self) -> std::cell::RefMut<'_, Vec<ErrorRecovery<usize, Token, LexicalError>>> {
        self.errors.borrow_mut()
    }

    /// Returns `true` if errors have been recorded.
    pub fn has_errors(&self) -> bool {
        !self.errors.borrow().is_empty()
    }

    /// Clears and returns all recorded errors.
    pub fn take_errors(&self) -> Vec<ErrorRecovery<usize, Token, LexicalError>> {
        std::mem::take(&mut *self.errors.borrow_mut())
    }

    /// Adds a new recoverable error to the list.
    pub fn push_error(&self, error: ErrorRecovery<usize, Token, LexicalError>) {
        self.errors.borrow_mut().push(error);
    }
}
