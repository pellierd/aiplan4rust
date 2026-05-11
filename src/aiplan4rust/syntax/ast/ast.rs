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
//! - A [`Tree<AstNode>`] representing the complete AST structure.
//! - A [`SymbolInterner`] used during parsing for deduplicating string content such as symbols.
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
//! - [`SymbolInterner`] for efficient symbol management.
//! - [`PreorderIter`] and [`PostorderIter`] for custom traversal.

use crate::aiplan4rust::interner::{InternerError, SelfInternerDisplay, SymbolInterner};
use crate::aiplan4rust::syntax::ast::error::AstError;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::{FastLineTable, SyntaxDisplay};
use crate::aiplan4rust::tree::{Node, NodeId, Tree};
use std::collections::HashMap;

use crate::aiplan4rust::lang::{LiteralId, RemapSymbol, SymbolId};
use crate::aiplan4rust::serialization::syntax::SyntaxSerializable;
use crate::aiplan4rust::serialization::SerializationError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::time::SystemTime;

/// Represents a complete Abstract Syntax Tree (AST) along with its context.
///
/// This struct owns the full syntax tree, a string interner to efficiently manage
/// and deduplicate symbolic strings encountered during parsing, and metadata such as
/// the source identifier and creation timestamp.
///
/// # Fields
///
/// - `syntax_tree`: The full AST represented as a tree of `AstNode` elements.
/// - `interner`: A `StringInterner` that stores and manages all unique strings used in the AST.
/// - `source_id`: A `Literal` serving as the interned identifier for the source from which this AST was parsed,
///   allowing efficient retrieval of the original source name without string duplication.
/// - `generated_at`: A `SystemTime` timestamp marking when this AST instance was created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ast {
    /// The complete syntax tree of the AST.
    syntax_tree: Tree<AstNode>,

    /// String interner used during parsing.
    interner: SymbolInterner,

    /// Interned identifier representing the source of this AST.
    source_id: LiteralId,

    /// Timestamp when the AST was generated.
    generated_at: SystemTime,
}

impl Default for Ast {
    /// Creates a new empty [`Ast`] instance initialized with debug values.
    ///
    /// The debug instance has:
    /// - An empty syntax tree.
    /// - An empty string interner.
    /// - A `source_id` set to `Literal::debug()` to indicate unknown source.
    /// - A generation timestamp set to the current system time.
    fn default() -> Self {
        Ast {
            syntax_tree: Tree::<AstNode>::new(),
            interner: SymbolInterner::new(),
            source_id: LiteralId::default(),
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
    /// * `interner` - A [`SymbolInterner`] for managing interned strings within the AST.
    /// * `source_id` - A identifier for the source of the AST (e.g., filename). This will be interned.
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
        syntax_tree: Tree<AstNode>,
        interner: SymbolInterner,
        source_id: LiteralId,
        generated_at: SystemTime,
    ) -> Self {
        Self {
            syntax_tree,
            interner,
            source_id,
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
    /// A reference to the internal [`Tree<AstNode>`], which holds all nodes
    /// allocated during parsing.
    ///
    /// # Example
    /// ```
    /// let ctx = ParseContext::new();
    /// let tree_ref = ctx.syntax_tree();
    /// assert!(tree_ref.is_empty());
    /// ```
    pub fn syntax_tree(&self) -> &Tree<AstNode> {
        &self.syntax_tree
    }

    /// Returns a mutable reference to the AST syntax tree.
    ///
    /// # Returns
    ///
    /// A mutable reference to the internal [`Tree<AstNode>`].
    pub fn syntax_tree_mut(&mut self) -> &mut Tree<AstNode> {
        &mut self.syntax_tree
    }

    /// Consumes and returns the syntax tree, replacing it with an empty tree.
    ///
    /// # Returns
    ///
    /// The owned [`Tree<AstNode>`] that was contained in the `Ast`.
    pub fn take_syntax_tree(&mut self) -> Tree<AstNode> {
        std::mem::take(&mut self.syntax_tree)
    }

    /// Returns a reference to the string interner.
    ///
    /// # Returns
    ///
    /// A shared reference to the [`SymbolInterner`] used in the AST.
    pub fn interner(&self) -> &SymbolInterner {
        &self.interner
    }

    /// Returns a mutable reference to the string interner.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [`SymbolInterner`] used in the AST or local storage.
    pub fn interner_mut(&mut self) -> &mut SymbolInterner {
        &mut self.interner
    }

    /// Consumes and returns the string interner, replacing it with an empty interner.
    ///
    /// # Returns
    ///
    /// The owned [`SymbolInterner`] that was contained in the `Ast`.
    pub fn take_interner(&mut self) -> SymbolInterner {
        std::mem::take(&mut self.interner)
    }

    /// Returns the interned identifier of the source that generated this AST.
    ///
    /// This `Literal` refers to a string stored in the interner, typically representing
    /// the filename or origin label of the AST (e.g., `"domain.pddl"` or `"stdin"`).
    ///
    /// If this method returns [`LiteralId::default()`], it typically means the source
    /// name is undefined or not set (e.g., in an empty or debug AST).
    ///
    /// To retrieve the actual string, use [`SymbolInterner::resolve_literal`] or
    /// [`SymbolInterner::try_resolve_literal`] with this value.
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
    pub fn source_id(&self) -> LiteralId {
        self.source_id
    }

    /// Attempts to resolve and return the source name as a string slice from the interner.
    ///
    /// This method uses the `Literal` identifier returned by `source_id()` to look up
    /// the actual source name string in the associated `StringInterner`.
    ///
    /// # Returns
    ///
    /// * `Ok(&str)` containing the resolved source name if successful.
    /// * `Err(InternerError)` if the `Literal` cannot be resolved, e.g., if the
    ///   source name is not set or invalid.
    pub fn try_source_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_literal(self.source_id())
    }

    /// Returns the source name as a string slice if it can be resolved from the interner.
    ///
    /// # Returns
    ///
    /// * `Some(&str)` containing the source name if it exists in the interner.
    /// * `None` if the source name cannot be found or is not set.
    pub fn source_name(&self) -> Option<&str> {
        self.interner.resolve_literal(self.source_id())
    }

    /// Returns the source name as a `String`.
    ///
    /// If the source name cannot be resolved, returns a fallback string
    /// of the form `"Unknown<{:?}>"` where the literal debug representation is included.
    ///
    /// # Returns
    ///
    /// A `String` representing the source name or a fallback placeholder.
    pub fn source_name_string(&self) -> String {
        self.source_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("unknown<{:?}>", self.source_id()))
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
    pub fn init_span(&mut self, fast_line_table: &FastLineTable) -> Result<(), AstError> {
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
        buf.push_str(&format!(";; Source: {}\n", self.source_id));
        buf.push_str(&format!(";; Generated at: {:?}\n\n", self.generated_at));

        // Append the PDDL syntax representation of the AST
        if let Some(root_id) = self.syntax_tree().root_id() {
            if let Some(root_node) = self.syntax_tree().get_node(root_id) {
                buf.push_str(&root_node.to_syntax_string(self.syntax_tree(), self.interner()));
            }
        }
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

impl SyntaxDisplay for Ast {
    /// Formats the `Ast` as a syntax-oriented string.
    ///
    /// This implementation delegates to the AST's syntax tree, rendering it
    /// with its internal interner and starting at indentation level 0.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the syntax string into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::fmt::Write;
    /// # let ast: Ast = todo!();
    /// let mut s = String::new();
    /// ast.fmt_syntax(&mut s).unwrap();
    /// println!("{}", s);
    /// ```
    fn fmt_syntax(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(root_id) = self.syntax_tree().root_id() {
            if let Some(root_node) = self.syntax_tree().get_node(root_id) {
                return root_node.fmt_syntax_with_indent(f, self.syntax_tree(), self.interner(), 0);
            }
        }
        Ok(())
    }
}

impl SelfInternerDisplay for Ast {
    /// Formats the `Ast` using its internal [`SymbolInterner`].
    ///
    /// This implementation delegates to the AST's syntax tree and resolves all
    /// interned identifiers using the AST's interner, producing a human-readable string.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the string into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::fmt::Write;
    /// # let ast: Ast = todo!();
    /// let mut s = String::new();
    /// ast.fmt_interner(&mut s).unwrap();
    /// println!("{}", s);
    /// ```
    fn fmt_interner(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(root_id) = self.syntax_tree().root_id() {
            if let Some(root_node) = self.syntax_tree().get_node(root_id) {
                // Utilise la méthode de rendu d'arbre que nous avons créée
                return root_node.fmt_with_interner(f, self.syntax_tree(), self.interner());
            }
        }
        Ok(())
    }
}

impl fmt::Display for Ast {
    /// Formats the AST for display.
    ///
    /// Prints the source name, generation timestamp, and the list of nodes with their syntax.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Abstract Syntax Tree:")?;
        writeln!(f, " - Source: {}", self.source_name_string())?;
        writeln!(f, " - Generated at: {:?}", self.generated_at)?;
        writeln!(f, " - Nodes:\n")?;
        self.fmt_interner(f)
    }
}

/// Implements serialization for the AST.
///
/// The AST is serialized using its `SyntaxDisplay` implementation, producing
/// a normalized, human-readable representation of the syntax tree. This allows
/// saving the AST to a file for later inspection, comparison, or re-parsing.
///
/// # Example
///
/// ```rust
/// # use crate::aiplan4rust::syntax::ast::Ast;
/// # let ast: Ast = todo!();
/// let serialized = ast.serialize_to_string().unwrap();
/// println!("{}", serialized);
/// ast.serialize_to_file("ast_normalized.pddl").unwrap();
/// ```
impl SyntaxSerializable for Ast {
    /// Serializes the AST to a string using its interner.
    ///
    /// Returns a normalized string representation of the AST.
    fn serialize_to_string(&self) -> Result<String, SerializationError> {
        Ok(self.to_syntax_string_with_comments())
    }
}

impl Tree<AstNode> {
    /// Remaps identifiers starting only from the root.
    pub fn remap_idents(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        // We retrieve the root ID. If it exists, we start the recursive remapping.
        if let Some(root_id) = self.root_id() {
            self.remap_idents_from(root_id, map)?;
        }
        Ok(())
    }

    /// Recursively remaps all identifiers in the subtree rooted at `id`.
    ///
    /// This method performs a depth-first traversal of the subtree and updates
    /// every node's semantic content using the provided mapping.
    ///
    /// # Parameters
    /// - `id`: The root `NodeId` of the subtree to process.
    /// - `map`: A hash map containing the identifier translations (Old -> New).
    ///
    /// # Errors
    /// Returns an `InternerError` if the remapping fails on any node.
    pub fn remap_idents_from(
        &mut self,
        id: NodeId,
        map: &HashMap<SymbolId, SymbolId>,
    ) -> Result<(), InternerError> {
        let mut stack = vec![id];
        while let Some(current_id) = stack.pop() {
            // Because we are in an impl for SyntaxTree<AstNode>,
            // the compiler knows that 'node' is an AstNode.
            if let Some(node) = self.get_node_mut(current_id) {
                // AstNode implements RemapIdents, so this call is valid.
                node.remap_symbol(map)?;

                for &child_id in node.children() {
                    stack.push(child_id);
                }
            }
        }
        Ok(())
    }
}
