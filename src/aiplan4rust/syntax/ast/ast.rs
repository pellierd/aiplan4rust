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
//! The tree can be traversed using built-in iterators:
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

use std::fmt::{self, Write as _};
use std::time::SystemTime;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::{
    AstNode,
    iterators::{PreorderIter, PostorderIter},
};
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::syntax::StringInterner;

/// A complete abstract syntax tree and its associated context.
///
/// This struct owns the entire syntax tree, the string interner used to deduplicate
/// symbolic strings, and metadata such as source origin and generation timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ast {
    /// Root node of the AST.
    root: Box<AstNode>,

    /// String interner used during parsing.
    context: StringInterner,

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
    /// - `context`: A [`StringInterner`] used to resolve interned content within the AST.
    /// - `source_name`: A human-readable label for the origin of the AST.
    /// - `generated_at`: A [`SystemTime`] indicating when the AST was built.
    pub fn new(
        root: Box<AstNode>,
        context: StringInterner,
        source_name: String,
        generated_at: SystemTime,
    ) -> Self {
        Self {
            root,
            context,
            source_name,
            generated_at,
        }
    }

    /// Returns a reference to the AST root node.
    pub fn root(&self) -> &Box<AstNode> {
        &self.root
    }

    /// Returns a mutable reference to the AST root node.
    pub fn root_mut(&mut self) -> &mut Box<AstNode> {
        &mut self.root
    }

    /// Returns a reference to the string interner used during parsing.
    pub fn context(&self) -> &StringInterner {
        &self.context
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
    pub fn preorder(&self) -> PreorderIter<'_> {
        PreorderIter::new(self.root())
    }

    /// Returns an iterator over the AST in postorder (children before node).
    pub fn postorder(&self) -> PostorderIter<'_> {
        PostorderIter::new(self.root())
    }

    pub fn get_str(&self, ident: Ident) -> Option<&str> {
        self.context.get_str(ident)
    }

    pub fn expect_str(&self, ident: Ident) -> Result<&str, ParserInternalError> {
        self.context.expect_str(ident)
    }
}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Abstract Syntax Tree:")?;
        writeln!(f, " - Source: {}", self.source_name)?;
        writeln!(f, " - Generated at: {:?}", self.generated_at)?;
        writeln!(f, " - Nodes:")?;

        for (node, depth) in self.preorder() {
            let indent = "  ".repeat(depth);
            let content = node.content().display_with_context(&self.context);
            writeln!(f, "{}- Kind: {:?}, Content: {}", indent, node.kind(), content)?;
        }

        Ok(())
    }
}
