//! High-Level Abstract Syntax Tree (AST) Representation for `aiplan4rust`
//!
//! This module defines the [`Ast`] type_checker, a container that encapsulates the entire
//! abstract syntax tree (AST) generated during parsing of PDDL or HDDL documents.
//! It centralizes the full syntactic structure and parsing metadata for downstream tasks
//! such as analysis, transformation, code generation, or pretty-printing.
//!
//! # Structure
//!
//! The [`Ast`] holds the following components:
//!
//! - A [`SyntaxTree<AstNode>`] representing the complete AST structure.
//! - A [`StringInterner`] used during parsing for deduplicating string content such as symbols.
//! - A human-readable [`source_name`] (e.g., a filename or label).
//! - A [`SystemTime`] timestamp recording when the AST was created.
//!
//! # Traversal
//!
//! The syntax tree can be traversed using built-in iterators:
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
//! let syntax_tree = SyntaxTree::<AstNode>::new(); // construction du syntax_tree selon ton AST
//!
//! let ast = Ast::new(syntax_tree, interner, "domain.pddl".into(), SystemTime::now());
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
//! - [`AstNode`] for details about individual syntax nodes.
//! - [`AstKind`] for syntax classification.
//! - [`StringInterner`] for efficient symbol management.
//! - [`PreorderIter`] and [`PostorderIter`] for custom traversal.

use crate::aiplan4rust::interner::{InternerDisplay, Literal, StringInterner};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::{FastLineTable, SyntaxDisplay};
use crate::aiplan4rust::syntax::ast::error::AstError;
use crate::aiplan4rust::syntax::tree::{SyntaxTree, NodeId, SyntaxNode};

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Debug;
use std::time::SystemTime;

/// A complete abstract syntax tree (AST) and its associated context.
///
/// This struct owns the entire syntax tree structure, the string interner used to deduplicate
/// symbolic strings, and metadata such as source origin and generation timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ast {
    /// The complete syntax tree of the AST.
    syntax_tree: SyntaxTree<AstNode>,

    /// String interner used during parsing.
    interner: StringInterner,

    /// Name or identifier for the source of the parsed AST.
    source_name: Literal,

    /// Timestamp of when the AST was created.
    generated_at: SystemTime,
}

impl Default for Ast {
    /// Creates a new empty [`Ast`] instance initialized with default values.
    ///
    /// The default instance has:
    /// - An empty syntax tree.
    /// - An empty string interner.
    /// - A `source_name` set to `Literal::default()` to indicate unknown source.
    /// - A generation timestamp set to the current system time.
    fn default() -> Self {
        Ast {
            syntax_tree: SyntaxTree::<AstNode>::new(),
            interner: StringInterner::new(),
            source_name: Literal::default(),
            generated_at: SystemTime::now(),
        }
    }
}

impl Ast {
    /// Creates a new [`Ast`] instance with the specified components.
    ///
    /// # Arguments
    ///
    /// * `syntax_tree` - The root syntax tree containing the AST nodes.
    /// * `interner` - A [`StringInterner`] for managing interned strings within the AST.
    /// * `source_name` - A human-readable identifier for the source of the AST (e.g., filename). This will be interned.
    /// * `generated_at` - A [`SystemTime`] timestamp marking when the AST was generated.
    ///
    /// # Returns
    ///
    /// A new `Ast` instance initialized with the provided syntax tree, interned source name,
    /// and generation timestamp.
    ///
    /// # Example
    /// ```
    /// use std::time::SystemTime;
    /// use your_crate::{Ast, StringInterner, SyntaxTree, AstNode};
    ///
    /// let mut interner = StringInterner::new();
    /// let syntax_tree = SyntaxTree::<AstNode>::new();
    /// let source_name = "example.pddl".to_string();
    /// let generated_at = SystemTime::now();
    ///
    /// let ast = Ast::new(syntax_tree, interner, source_name, generated_at);
    /// ```
    pub fn new(
        syntax_tree: SyntaxTree<AstNode>,
        mut interner: StringInterner,
        source_name: String,
        generated_at: SystemTime,
    ) -> Self {
        let source_literal = interner.intern_literal(source_name);
        Self {
            syntax_tree,
            interner,
            source_name: source_literal,
            generated_at,
        }
    }

    /// Returns a shared reference to the syntax tree (AST arena).
    ///
    /// This allows read-only access to the internal arena-allocated
    /// abstract syntax tree containing all parsed [`AstNode`]s.
    ///
    /// # Returns
    ///
    /// A reference to the internal [`SyntaxTree<AstNode>`], which holds all nodes
    /// allocated during parsing.
    ///
    /// # Example
    /// ```
    /// let ctx = ParseContext::new();
    /// let tree_ref = ctx.syntax_tree();
    /// assert!(tree_ref.is_empty());
    /// ```
    pub fn syntax_tree(&self) -> &SyntaxTree<AstNode> {
        &self.syntax_tree
    }

    /// Returns a mutable reference to the AST syntax tree.
    ///
    /// # Returns
    ///
    /// A mutable reference to the internal [`SyntaxTree<AstNode>`].
    pub fn syntax_tree_mut(&mut self) -> &mut SyntaxTree<AstNode> {
        &mut self.syntax_tree
    }

    /// Consumes and returns the syntax tree, replacing it with an empty tree.
    ///
    /// # Returns
    ///
    /// The owned [`SyntaxTree<AstNode>`] that was contained in the `Ast`.
    pub fn take_syntax_tree(&mut self) -> SyntaxTree<AstNode> {
        std::mem::take(&mut self.syntax_tree)
    }

    /// Returns a reference to the string interner.
    ///
    /// # Returns
    ///
    /// A shared reference to the [`StringInterner`] used in the AST.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Returns a mutable reference to the string interner.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [`StringInterner`] used in the AST or local storage.
    pub fn interner_mut(&mut self) -> &mut StringInterner {
        &mut self.interner
    }

    /// Consumes and returns the string interner, replacing it with an empty interner.
    ///
    /// # Returns
    ///
    /// The owned [`StringInterner`] that was contained in the `Ast`.
    pub fn take_interner(&mut self) -> StringInterner {
        std::mem::take(&mut self.interner)
    }

    /// Returns the interned identifier of the source that generated this AST.
    ///
    /// This `Literal` refers to a string stored in the interner, typically representing
    /// the filename or origin label of the AST (e.g., `"domain.pddl"` or `"stdin"`).
    ///
    /// If this method returns [`Literal::default()`], it typically means the source
    /// name is undefined or not set (e.g., in an empty or default AST).
    ///
    /// To retrieve the actual string, use [`StringInterner::resolve_literal`] or
    /// [`StringInterner::try_resolve_literal`] with this value.
    ///
    /// # Returns
    ///
    /// A `Literal` representing the interned source name.
    ///
    /// # Example
    ///
    /// ```
    /// let source_id = ast.source_name();
    /// if let Some(name) = ast.interner().resolve_literal(source_id) {
    ///     println!("Source: {}", name);
    /// } else {
    ///     println!("Unknown source");
    /// }
    /// ```
    pub fn source_name(&self) -> Literal {
        self.source_name
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
        for (id, _) in self.syntax_tree.preorder_from(node_id).with_id() {
            let node = self.syntax_tree.get_node(id)?;
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
        self.syntax_tree
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
    ) -> Result<(), AstError> {
        if !self.syntax_tree().is_empty() {
            let mut stack = vec![self.syntax_tree().try_root_id()?];
            while let Some(node_id) = stack.pop() {
                // Get a mutable reference to the current node
                let node = self.syntax_tree_mut().try_node_mut(node_id)?;

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
        self.syntax_tree().to_string_with_interner(self.interner())
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
        self.syntax_tree().to_syntax_string(self.interner())
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
        buf.push_str(&self.syntax_tree().to_syntax_string(self.interner()));

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
        node.to_syntax_string(self.syntax_tree(), self.interner())
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
        node.to_string_with_interner(self.syntax_tree(), self.interner())
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
        self.syntax_tree().fmt_with_interner(f, self.interner())?;
        Ok(())
    }
}
