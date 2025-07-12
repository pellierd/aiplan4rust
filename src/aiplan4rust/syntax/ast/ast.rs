//! High-Level Abstract Syntax Tree (AST) Representation for `aiplan4rust`
//!
//! This module defines the [`Ast`] type, a container that encapsulates the core components
//! of an abstract syntax tree (AST) generated during the parsing phase of PDDL or HDDL documents.
//! It centralizes both syntactic structure and parsing metadata for downstream tasks
//! such as analysis, transformation, code generation, or pretty-printing.
//!
//! # Structure
//!
//! The [`Ast`] holds the following components:
//!
//! - A [`Box<AstNode>`] representing the root of the syntax tree.
//! - A [`StringInterner`] used during parsing for deduplicating string content such as symbols.
//! - A human-readable [`source_name`] (e.g., a filename or label).
//! - A [`SystemTime`] timestamp recording when the AST was created.
//!
//! # Traversal
//!
//! The tree can be traversed using built-in iter:
//!
//! - [`Ast::preorder()`] — depth-first traversal where the parent is visited before its children.
//! - [`Ast::postorder()`] — depth-first traversal where the children are visited before the parent.
//!
//! These provide the basis for semantic analysis, validation, evaluation, and more.
//!
//! # Example
//!
//! ```rust
//! use std::time::SystemTime;
//! use aiplan4rust::syntax::{Ast, AstNode, AstKind, AstContent, StringInterner};
//!
//! let mut interner = StringInterner::default();
//! let root = Box::new(AstNode::new(AstKind::Symbol, AstContent::interned("move", &mut interner)));
//!
//! let ast = Ast::new(root, interner, "domain.pddl".into(), SystemTime::now());
//!
//! for (node, depth) in ast.preorder() {
//!     println!("{:indent$}- {:?}", "", node.kind(), indent = depth * 2);
//! }
//! ```
//!
//! # Use Cases
//!
//! - Parser output for PDDL/HDDL domains and problems.
//! - Static analysis tools.
//! - Code generation pipelines.
//! - Visual AST inspection or pretty-printers.
//! - Intermediary format for serialization/deserialization.
//!
//! # See Also
//!
//! - [`AstNode`] for details about individual tree nodes.
//! - [`AstKind`] for node classification.
//! - [`StringInterner`] for efficient symbol management.
//! - [`PreorderIter`] and [`PostorderIter`] for custom traversal.

use std::fmt;
use std::time::SystemTime;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::{FastLineTable, PlanningSyntaxDisplay};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::tree::{NodeId, Arena};

/// A complete abstract syntax tree and its associated context.
///
/// This struct owns the entire syntax tree, the string interner used to deduplicate
/// symbolic strings, and metadata such as source origin and generation timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ast {

    /// Root node of the AST.
    arena: Arena<AstNode>,

    /// String interner used during parsing.
    interner: StringInterner,

    /// Name or identifier for the source of the parsed AST.
    source_name: String,

    /// Timestamp of when the AST was created.
    generated_at: SystemTime,
}

impl Ast {
    /// Creates a new [`Ast`] instance.
    ///
    /// # Arguments
    ///
    /// - `root`: The root node of the AST.
    /// - `interner`: A [`StringInterner`] used to resolve interned content within the AST.
    /// - `source_name`: A human-readable label for the origin of the AST.
    /// - `generated_at`: A [`SystemTime`] indicating when the AST was built.
    pub fn new(
        arena: Arena<AstNode>,
        interner: StringInterner,
        source_name: String,
        generated_at: SystemTime,
    ) -> Self {
        Self {
            arena,
            interner,
            source_name,
            generated_at,
        }
    }
    pub fn default() -> Self {
        Ast {
            arena: Arena::<AstNode>::new(),         // suppose que AstNode impl Default
            interner: StringInterner::new(),             // interner vide
            source_name: String::new(),                  // chaîne vide par défaut
            generated_at: SystemTime::now(),             // horodatage actuel
        }
    }

    /// Returns a reference to the AST root node.
    pub fn arena(&self) -> &Arena<AstNode> {
        &self.arena
    }

    /// Returns a mutable reference to the AST root node.
    pub fn arena_mut(&mut self) -> &mut Arena<AstNode> {
        &mut self.arena
    }

    /// Consumes and returns the root AST node.
    pub fn take_arena(&mut self) -> Arena<AstNode> {
        std::mem::take(&mut self.arena)
    }

    /// Returns a reference to the string interner used during parsing.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Consumes and returns the string interner.
    pub fn take_interner(&mut self) -> StringInterner {
        std::mem::take(&mut self.interner)
    }

    /// Returns the name or label of the source that generated this AST.
    pub fn source_name(&self) -> &String {
        &self.source_name
    }

    /// Returns the timestamp indicating when the AST was generated.
    pub fn generated_at(&self) -> SystemTime {
        self.generated_at
    }

    /// Returns an iterator over the AST in preorder (node before children).
    /*pub fn preorder(&self) -> PreorderIter<'_> {
        PreorderIter::new(self.root())
    }*/

    /// Returns an iterator over the AST in postorder (children before node).
    /*pub fn postorder(&self) -> PostorderIter<'_> {
        PostorderIter::new(self.root())
    }*/

    /// Attempts to resolve an interned identifier to its corresponding string slice.
    ///
    /// Returns `Some(&str)` if the identifier exists in the interner, or `None` otherwise.
    ///
    /// # Arguments
    /// * `ident` - The interned identifier to resolve.
    ///
    /// # Examples
    /// ```rust
    /// if let Some(name) = ast.resolve(some_ident) {
    ///     println!("Resolved name: {}", name);
    /// }
    /// ```
    pub fn resolve(&self, ident: Ident) -> Option<&str> {
        self.interner.resolve(ident)
    }

    /// Attempts to resolve an interned identifier to its corresponding string slice,
    /// returning an error if the identifier is not found.
    ///
    /// # Arguments
    /// * `ident` - The interned identifier to resolve.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if the identifier is not present in the interner.
    ///
    /// # Examples
    /// ```rust
    /// match ast.try_resolve(some_ident) {
    ///     Ok(name) => println!("Resolved name: {}", name),
    ///     Err(e) => eprintln!("Failed to resolve ident: {}", e),
    /// }
    /// ```
    pub fn try_resolve(&self, ident: Ident) -> Result<&str, ParserInternalError> {
        self.interner.try_resolve(ident)
    }


    /// Finds the first node ID of the specified kind in the subtree rooted at `node_id`.
    ///
    /// # Arguments
    /// * `node_id` - The root node ID of the subtree to search.
    /// * `kind` - The `AstKind` to find.
    ///
    /// # Returns
    /// * `Some(NodeId)` if a matching node is found.
    /// * `None` otherwise.
    pub fn find_node_id_of_kind_from(
        &self,
        node_id: NodeId,
        kind: AstKind,
    ) -> Option<NodeId> {
        for id in self.arena.preorder_ids_from(node_id) {
            let node = self.arena.get_node(id)?;
            if node.kind() == kind {
                return Some(id);
            }
        }
        None
    }

    /// Finds the first node ID of the specified kind in the entire AST.
    ///
    /// # Returns
    /// * `Some(NodeId)` if a matching node is found.
    /// * `None` otherwise.
    pub fn find_node_id_of_kind(&self, kind: AstKind) -> Option<NodeId> {
        self.arena.root_id().and_then(|root_id| {
            self.find_node_id_of_kind_from(root_id, kind)
        })
    }

    /// Recursively sets the start and end positions (line and column) for each AST node.
    ///
    /// This function traverses the AST in a pre-order fashion, updating each node's span information
    /// with precise line and column numbers obtained from the provided `FastLineTable`.
    ///
    /// # Arguments
    /// * `fast_line_table` - A reference to a `FastLineTable` used to convert byte offsets to line and column positions.
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if any node cannot be accessed mutably.
    pub fn init_span(
        &mut self,
        fast_line_table: &FastLineTable,
    ) -> Result<(), ParserInternalError> {

        if !self.arena().is_empty() {
            let mut stack = vec![self.arena().try_root_id()?];
            while let Some(node_id) = stack.pop() {
                // Get a mutable reference to the current node
                let node = self.arena_mut().try_node_mut(node_id)?;

                // Initialize start position (line, column) using the fast_line_table
                let (line_start, col_start) = fast_line_table.get_position(node.span().start());
                node.span_mut().set_start_line(line_start);
                node.span_mut().set_start_column(col_start);

                // Initialize end position (line, column) using the fast_line_table
                let (line_end, col_end) = fast_line_table.get_position(node.span().end());
                node.span_mut().set_end_line(line_end);
                node.span_mut().set_end_column(col_end);

                // Push the children onto the stack in reverse order for pre-order traversal
                for &child_id in node.children().iter().rev() {
                    stack.push(child_id);
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Abstract Syntax Tree:")?;
        writeln!(f, " - Source: {}", self.source_name)?;
        writeln!(f, " - Generated at: {:?}", self.generated_at)?;
        writeln!(f, " - Nodes:")?;
        self.arena().fmt_planning_syntax(f, self.interner())?;



        /*for (idx, node) in self.arena().nodes.iter().enumerate() {
            // Transforme les enfants (Vec<NodeId>) en string "id1, id2, id3"
            let children_str = node.children()
                .iter()
                .map(|child_id| child_id.to_string())  // si NodeId est un nouveau-type autour de usize, adapte ici
                .collect::<Vec<_>>()
                .join(", ");
            let content = node.content().to_string_with_interner(&self.interner);
            writeln!(f, "- Id: {}, Kind: {:?}, Content: {} , childen: {}", idx, node.kind(), content, children_str)?;
        }*/


        Ok(())
    }
}
