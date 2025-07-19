//! High-Level Abstract Syntax Tree (AST) Representation for `aiplan4rust`
//!
//! This module defines the [`Ast`] type, a container that encapsulates the core components
//! of an abstract syntax arena (AST) generated during the parsing phase of PDDL or HDDL documents.
//! It centralizes both syntactic structure and parsing metadata for downstream tasks
//! such as analysis, transformation, code generation, or pretty-printing.
//!
//! # Structure
//!
//! The [`Ast`] holds the following components:
//!
//! - An [`Arena<AstNode>`] representing the root of the syntax arena.
//! - A [`StringInterner`] used during parsing for deduplicating string content such as symbols.
//! - A human-readable [`source_name`] (e.g., a filename or label).
//! - A [`SystemTime`] timestamp recording when the AST was created.
//!
//! # Traversal
//!
//! The arena can be traversed using built-in iterators:
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
//! let root = Arena::<AstNode>::new(); // construction du root à adapter selon ton AST
//!
//! let ast = Ast::new(root, interner, "domain.pddl".into(), SystemTime::now());
//!
//! for (syntax, depth) in ast.preorder() {
//!     println!("{:indent$}- {:?}", "", syntax.kind(), indent = depth * 2);
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
//! - [`AstNode`] for details about individual arena nodes.
//! - [`AstKind`] for syntax classification.
//! - [`StringInterner`] for efficient symbol management.
//! - [`PreorderIter`] and [`PostorderIter`] for custom traversal.

use crate::aiplan4rust::arena::{Arena, ArenaNode, NodeId};
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::{FastLineTable, SyntaxDisplay};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Debug;
use std::time::SystemTime;

/// A complete abstract syntax arena and its associated context.
///
/// This struct owns the entire syntax arena, the string interner used to deduplicate
/// symbolic strings, and metadata such as source origin and generation timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ast {
    /// Root syntax of the AST.
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
    /// * `arena` - The root syntax arena of the AST.
    /// * `interner` - A [`StringInterner`] used to resolve interned content within the AST.
    /// * `source_name` - A human-readable label for the origin of the AST.
    /// * `generated_at` - A [`SystemTime`] indicating when the AST was built.
    ///
    /// # Returns
    ///
    /// A new `Ast` instance containing the provided components.
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

    /// Creates a new empty [`Ast`] instance with default values.
    ///
    /// - The arena is empty.
    /// - The interner is empty.
    /// - The source name is an empty string.
    /// - The generation timestamp is set to the current system time.
    ///
    /// # Returns
    ///
    /// A default `Ast` instance.
    pub fn default() -> Self {
        Ast {
            arena: Arena::<AstNode>::new(),
            interner: StringInterner::new(),
            source_name: String::new(),
            generated_at: SystemTime::now(),
        }
    }

    /// Returns a reference to the AST arena.
    ///
    /// # Returns
    ///
    /// A shared reference to the internal [`Arena<AstNode>`].
    pub fn arena(&self) -> &Arena<AstNode> {
        &self.arena
    }

    /// Returns a mutable reference to the AST arena.
    ///
    /// # Returns
    ///
    /// A mutable reference to the internal [`Arena<AstNode>`].
    pub fn arena_mut(&mut self) -> &mut Arena<AstNode> {
        &mut self.arena
    }

    /// Consumes and returns the arena, replacing it with an empty arena.
    ///
    /// # Returns
    ///
    /// The owned [`Arena<AstNode>`] that was contained in the `Ast`.
    pub fn take_arena(&mut self) -> Arena<AstNode> {
        std::mem::take(&mut self.arena)
    }

    /// Returns a reference to the string interner.
    ///
    /// # Returns
    ///
    /// A shared reference to the [`StringInterner`] used in the AST.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Consumes and returns the string interner, replacing it with an empty interner.
    ///
    /// # Returns
    ///
    /// The owned [`StringInterner`] that was contained in the `Ast`.
    pub fn take_interner(&mut self) -> StringInterner {
        std::mem::take(&mut self.interner)
    }

    /// Returns the name or label of the source that generated this AST.
    ///
    /// # Returns
    ///
    /// A reference to the source name string.
    pub fn source_name(&self) -> &String {
        &self.source_name
    }

    /// Returns the timestamp indicating when the AST was generated.
    ///
    /// # Returns
    ///
    /// A [`SystemTime`] timestamp.
    pub fn generated_at(&self) -> SystemTime {
        self.generated_at
    }

    /// Finds the first syntax node ID of the specified kind in the subtree rooted at `node_id`.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The root node ID of the subtree to search.
    /// * `kind` - The [`AstKind`] to find.
    ///
    /// # Returns
    ///
    /// * `Some(NodeId)` if a matching node is found.
    /// * `None` if no matching node is found.
    pub fn find_node_id_of_kind_from(&self, node_id: NodeId, kind: AstKind) -> Option<NodeId> {
        for id in self.arena.preorder_ids_from(node_id) {
            let node = self.arena.get_node(id)?;
            if node.kind() == kind {
                return Some(id);
            }
        }
        None
    }

    /// Finds the first syntax node ID of the specified kind in the entire AST.
    ///
    /// # Arguments
    ///
    /// * `kind` - The [`AstKind`] to find.
    ///
    /// # Returns
    ///
    /// * `Some(NodeId)` if a matching node is found.
    /// * `None` if no matching node is found.
    pub fn find_node_id_of_kind(&self, kind: AstKind) -> Option<NodeId> {
        self.arena
            .root_id()
            .and_then(|root_id| self.find_node_id_of_kind_from(root_id, kind))
    }

    /// Recursively initializes the span (start and end positions: line and column)
    /// for each AST syntax node based on a [`FastLineTable`].
    ///
    /// This method traverses the AST in pre-order and updates each node’s span
    /// with precise line and column numbers obtained from the `FastLineTable`.
    ///
    /// # Arguments
    ///
    /// * `fast_line_table` - Reference to a [`FastLineTable`] to convert byte offsets to line/column positions.
    ///
    /// # Errors
    ///
    /// Returns a [`AiplanError`] if a syntax node cannot be accessed mutably.
    pub fn init_span(
        &mut self,
        fast_line_table: &FastLineTable,
    ) -> Result<(), AiplanError> {
        if !self.arena().is_empty() {
            let mut stack = vec![self.arena().try_root_id()?];
            while let Some(node_id) = stack.pop() {
                // Get a mutable reference to the current node
                let node = self.arena_mut().try_node_mut(node_id)?;

                // Initialize start position (line, column)
                let (line_start, col_start) = fast_line_table.get_position(node.span().start());
                node.span_mut().set_start_line(line_start);
                node.span_mut().set_start_column(col_start);

                // Initialize end position (line, column)
                let (line_end, col_end) = fast_line_table.get_position(node.span().end());
                node.span_mut().set_end_line(line_end);
                node.span_mut().set_end_column(col_end);

                // Push children in reverse order to preserve pre-order traversal
                for &child_id in node.children().iter().rev() {
                    stack.push(child_id);
                }
            }
        }
        Ok(())
    }

    /// Returns a string representation of the AST with symbols resolved
    /// using the associated [`StringInterner`].
    ///
    /// This method prints the AST nodes in a detailed debug-like format,
    /// showing node names along with their interned strings for clarity.
    ///
    /// # Example
    ///
    /// ```
    /// let s = ast.to_string_with_interner();
    /// println!("{}", s);
    /// ```
    pub fn to_string_with_interner(&self) -> String {
        self.arena().to_string_with_interner(self.interner())
    }

    /// Returns a string representation of the AST formatted as PDDL syntax.
    ///
    /// This method produces a PDDL-compliant serialization of the AST,
    /// suitable for outputting a valid PDDL domain or problem description.
    ///
    /// # Example
    ///
    /// ```
    /// let pddl = ast.to_syntax_string();
    /// println!("{}", pddl);
    /// ```
    pub fn to_syntax_string(&self) -> String {
        self.arena().to_syntax_string(self.interner())
    }

    /// Returns a string representing the AST formatted as PDDL syntax,
    /// including metadata as PDDL-style comments.
    ///
    /// This string includes the source name and generation timestamp
    /// as comments at the beginning of the output, followed by the
    /// pretty-printed AST syntax.
    ///
    /// # Example
    ///
    /// ```
    /// let pddl_str = ast.to_syntax_string_with_comments();
    /// println!("{}", pddl_str);
    /// ```
    ///
    pub fn to_syntax_string_with_comments(&self) -> String {
        let mut buf = String::new();

        // Add metadata as PDDL-style comments
        buf.push_str(&format!(";; Source: {}\n", self.source_name));
        buf.push_str(&format!(";; Generated at: {:?}\n\n", self.generated_at));

        // Append the PDDL syntax representation of the AST
        buf.push_str(&self.arena().to_syntax_string(self.interner()));

        buf
    }

    /// Converts an AST node to its syntax string representation using this AST's arena and interner.
    ///
    /// # Arguments
    /// * `node` - The `AstNode` to convert.
    ///
    /// # Returns
    /// A string representing the node's syntax (i.e., how it appears in the source).
    pub fn to_syntax_string_from(&self, node: &AstNode) -> String {
        // Delegate to the node's `to_syntax_string` method with the current arena and interner
        node.to_syntax_string(self.arena(), self.interner())
    }

    /// Converts an AST node to a string using the interner for resolving identifiers.
    ///
    /// # Arguments
    /// * `node` - The `AstNode` to convert.
    ///
    /// # Returns
    /// A string with interned names resolved for better readability.
    pub fn to_string_interner_from(&self, node: &AstNode) -> String {
        // Delegate to the node's `to_string_with_interner` method with the current arena and interner
        node.to_string_with_interner(self.arena(), self.interner())
    }

}

impl fmt::Display for Ast {
    /// Formats the AST for display.
    ///
    /// Prints the source name, generation timestamp, and the list of nodes with their syntax.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Abstract Syntax Tree:")?;
        writeln!(f, " - Source: {}", self.source_name)?;
        writeln!(f, " - Generated at: {:?}", self.generated_at)?;
        writeln!(f, " - Nodes:\n")?;
        self.arena().fmt_with_interner(f, self.interner())?;
        Ok(())
    }
}
