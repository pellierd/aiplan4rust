//! Module providing parsing context used by the LALRPOP-generated parser.
//!
//! The `ParseContext` struct acts as the central structure during parsing,
//! containing mutable access to:
//! - the [`Arena`] storing the AST nodes,
//! - the [`StringInterner`] for deduplicating strings,
//! - the list of lexical or syntactic errors collected during parsing.
//!
//! This context is passed around throughout the parsing process and is
//! responsible for managing node allocation, error recording, and interning
//! strings.
//!
//! # Components
//! - [`ParseContext`] manages arena-allocated AST nodes and error collection.
//! - [`Arena`] is a generic arena allocator for tree structures.
//! - [`StringInterner`] reduces memory usage by deduplicating strings.
//!
//! This module is designed to be used internally by the parser.

use std::cell::RefCell;
use lalrpop_util::ErrorRecovery;

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::syntax::lexer::{LexicalError, Token};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::arena::{NodeId, Arena, ArenaNode};

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
///
/// # Example
/// ```
/// let ctx = ParseContext::new();
/// let id = ctx.intern("var".to_string());
/// ```
pub struct ParseContext {
    /// Interns strings to deduplicate identifiers and literals.
    interner: RefCell<StringInterner>,

    /// Stores all AST nodes allocated during parsing.
    arena: RefCell<Arena<AstNode>>,

    /// Stores recoverable errors encountered during parsing.
    errors: RefCell<Vec<ErrorRecovery<usize, Token, LexicalError>>>,
}

impl ParseContext {
    /// Creates a new, empty `ParseContext`.
    ///
    /// This initializes:
    /// - an empty [`Arena`] for storing AST nodes,
    /// - a fresh [`StringInterner`] for deduplicating strings,
    /// - an empty error list.
    ///
    /// # Example
    /// ```
    /// let ctx = ParseContext::new();
    /// assert!(ctx.root_id().is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            interner: RefCell::new(StringInterner::new()),
            arena: RefCell::new(Arena::empty()),
            errors: RefCell::new(Vec::new()),
        }
    }

    /// Provides read-only access to the arena.
    ///
    /// Use this if you want to inspect nodes without modifying them.
    ///
    /// # Returns
    /// A shared reference to the arena.
    pub fn borrow_arena(&self) -> std::cell::Ref<'_, Arena<AstNode>> {
        self.arena.borrow()
    }

    /// Provides mutable access to the arena.
    ///
    /// Use this if you need to mutate nodes or the arena structure.
    ///
    /// # Panics
    /// Panics if another borrow (mutable or immutable) is still active.
    ///
    /// # Returns
    /// A mutable reference to the arena.
    pub fn borrow_arena_mut(&self) -> std::cell::RefMut<'_, Arena<AstNode>> {
        self.arena.borrow_mut()
    }

    /// Takes ownership of the arena and replaces it with a new empty arena.
    ///
    /// Use this to extract all allocated nodes after parsing.
    ///
    /// # Returns
    /// The previous arena containing all nodes.
    pub fn take_arena(&self) -> Arena<AstNode> {
        std::mem::take(&mut *self.arena.borrow_mut())
    }

    /// Sets the root node ID in the arena.
    ///
    /// # Arguments
    /// * `root_id` - The ID of the node to set as root.
    ///
    /// # Errors
    /// Returns [`ParserInternalError`] if the node ID does not exist.
    pub fn set_root_id(&self, root_id: NodeId) -> Result<(), ParserInternalError> {
        self.arena.borrow_mut().set_root_id(root_id)
    }

    /// Returns the ID of the root node, if any.
    ///
    /// # Returns
    /// An `Option` containing the `NodeId` of the root node.
    pub fn root_id(&self) -> Option<NodeId> {
        self.arena.borrow().root_id()
    }

    /// Allocates a new AST node in the arena.
    ///
    /// This inserts the node, updates all children to reference it as their parent,
    /// and returns the newly assigned `NodeId`.
    ///
    /// # Arguments
    /// * `kind` - The kind of AST node.
    /// * `content` - The node content.
    /// * `children` - The list of child node IDs.
    /// * `start` - Start offset in the input.
    /// * `end` - End offset in the input.
    ///
    /// # Returns
    /// The `NodeId` of the allocated node, or an error if allocation fails.
    pub fn alloc_node(
        &self,
        kind: AstKind,
        content: AstContent,
        children: Vec<NodeId>,
        start: usize,
        end: usize,
    ) -> Result<NodeId, ParserInternalError> {
        let span = Span::new(start, end);

        // 1) Create the node with the children (parent initially None)
        let node = AstNode::new(kind, content, children, span, None);

        let mut arena = self.arena.borrow_mut();

        // 2) Allocate it and get its NodeId
        let node_id = arena.alloc(node);

        // 3) Clone children list before releasing borrow
        let children_ids = {
            let stored = arena.try_node(node_id)?;
            stored.children().to_vec()
        };

        // 4) Update each child to reference this node as parent
        for child in &children_ids {
            if let Some(child_node) = arena.get_node_mut(*child) {
                child_node.set_parent(Some(node_id));
            }
        }

        Ok(node_id)
    }

    /// Merges the children of one `TypedList` node into another.
    ///
    /// All children of `next` are moved into `typed_list` and updated
    /// to have `typed_list` as their parent.
    ///
    /// # Arguments
    /// * `typed_list` - The `NodeId` receiving the children.
    /// * `next` - The `NodeId` whose children will be moved.
    ///
    /// # Returns
    /// The updated `typed_list` `NodeId`.
    pub fn merge_typed_list(
        &mut self,
        typed_list: NodeId,
        next: NodeId,
    ) -> Result<NodeId, ParserInternalError> {
        let mut arena = self.borrow_arena_mut();

        // 1) Drain children from `next`
        let next_children = {
            let next_node = arena.try_node_mut(next)?;
            std::mem::take(next_node.children_mut())
        };

        // 2) Update parent references
        for child in &next_children {
            let child_node = arena.try_node_mut(*child)?;
            child_node.set_parent(Some(typed_list));
        }

        // 3) Append them to `typed_list`
        let typed_list_node = arena.try_node_mut(typed_list)?;
        typed_list_node.children_mut().extend(next_children);

        Ok(typed_list)
    }

    // String interner manipulation

    /// Provides read-only access to the `StringInterner`.
    ///
    /// # Returns
    /// A shared reference to the `StringInterner`.
    pub fn borrow_interner(&self) -> std::cell::Ref<'_, StringInterner> {
        self.interner.borrow()
    }

    /// Provides mutable access to the `StringInterner`.
    ///
    /// # Returns
    /// A mutable reference to the `StringInterner`.
    pub fn borrow_interner_mut(&self) -> std::cell::RefMut<'_, StringInterner> {
        self.interner.borrow_mut()
    }

    /// Takes ownership of the current `StringInterner`, replacing it with a fresh one.
    ///
    /// # Returns
    /// The previous `StringInterner`.
    pub fn take_interner(&self) -> StringInterner {
        std::mem::take(&mut *self.interner.borrow_mut())
    }

    /// Interns a string and returns its unique identifier.
    ///
    /// # Arguments
    /// * `s` - The string to intern.
    ///
    /// # Returns
    /// An `Ident` representing the interned string.
    pub fn intern(&self, s: String) -> Ident {
        self.interner.borrow_mut().intern(s)
    }

    // Error management

    /// Provides read-only access to the list of accumulated errors.
    ///
    /// # Returns
    /// A shared reference to the error list.
    pub fn borrow_errors(&self) -> std::cell::Ref<'_, Vec<ErrorRecovery<usize, Token, LexicalError>>> {
        self.errors.borrow()
    }

    /// Provides mutable access to the list of accumulated errors.
    ///
    /// # Returns
    /// A mutable reference to the error list.
    pub fn borrow_errors_mut(&self) -> std::cell::RefMut<'_, Vec<ErrorRecovery<usize, Token, LexicalError>>> {
        self.errors.borrow_mut()
    }

    /// Returns `true` if any error has been recorded.
    ///
    /// # Returns
    /// `true` if there are errors, `false` otherwise.
    pub fn has_errors(&self) -> bool {
        !self.errors.borrow().is_empty()
    }

    /// Takes ownership of all errors, replacing the list with an empty vector.
    ///
    /// # Returns
    /// The previous list of errors.
    pub fn take_errors(&self) -> Vec<ErrorRecovery<usize, Token, LexicalError>> {
        std::mem::take(&mut *self.errors.borrow_mut())
    }

    /// Adds a new error to the error list.
    ///
    /// # Arguments
    /// * `error` - The error to record.
    pub fn push_error(&self, error: ErrorRecovery<usize, Token, LexicalError>) {
        self.errors.borrow_mut().push(error);
    }
}
