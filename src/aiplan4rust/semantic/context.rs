//! Semantic Analysis Module
//!
//! This module provides the core components for semantic analysis within the compiler pipeline.
//! It centralizes semantic-related data structures, error handling, and processing logic.
//!
//! # Overview
//!
//! The semantic analysis phase verifies the correctness of the syntax tree beyond parsing,
//! including symbol resolution, semantic checks, and type checking. This module defines:
//!
//! - **`Context`**: The main semantic context structure holding the semantically enriched AST,
//!   symbol table, semantic requirements, and associated metadata.
//! - **Error Types**: Semantic-related error enumerations and specific error types such as
//!   `SemanticError`, `UnexpectedNodeKindError`, and `InvalidNodeArityError`.
//! - **Symbol Table Management**: Structures and utilities for symbol resolution during semantic analysis.
//! - **Analyzer Components**: Components that perform semantic checking and type checking.
//!
//! # Submodules
//!
//! - `analyzer`: Implements the core semantic analyzer logic.
//! - `symbol`: Contains symbol representations used in semantic processing.
//! - `analyzer_result`: Defines result types used by analyzers.
//! - `symbol_table`: Manages the symbol table data structures.
//! - `checks`: Implements various semantic checks.
//! - `context`: Defines the `Context` structure and its associated functionality.
//! - `error`: Contains semantic error definitions and error handling utilities.
//! - `type_checker`: Handles type checking logic and related operations.
//!
//! # Usage Example
//!
//! ```rust
//! use aiplan4rust::semantic::{Analyzer, Context, SemanticError};
//!
//! // Assume `ast` is a parsed abstract syntax tree.
//! let mut ast = ...;
//!
//! // Convert the AST into a semantic context, performing semantic analysis.
//! let context = Context::try_from(&mut ast)?;
//!
//! // Use the semantic analyzer to perform further checks or transformations.
//! let analyzer = Analyzer::new();
//! let result = analyzer.analyze(&context)?;
//! ```
//!
//! # Error Handling
//!
//! The module uses `SemanticError` as a unified error type that encompasses
//! errors from various semantic subcomponents such as symbol table errors,
//! type checking errors, and unexpected AST node errors.
//!
//! This design enables streamlined error propagation and reporting during
//! semantic analysis.

use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::{SemanticError, SymbolTable};
use crate::aiplan4rust::syntax::ast::{Ast, AstNode, AstKind};
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::serialization::serde::SerdeSerializable;
use crate::aiplan4rust::syntax::tree::{NodeId, SyntaxTree};

use std::collections::HashSet;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

/// Semantic context representing the result of semantic analysis.
///
/// This structure contains a semantically enriched abstract syntax tree (AST),
/// a symbol table, the set of semantic requirements, and metadata such as
/// the source file name and the timestamp of generation.
///
/// It serves as the main interface between parsing and subsequent phases like
/// type checking, optimization, or code generation.
///
/// # Examples
///
/// ```rust
/// use aiplan4rust::semantic::Context;
/// // Assume `ast` is obtained from parsing
/// // let context = Context::try_from(&mut ast)?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// The annotated syntax tree stored as an arena of AST nodes.
    syntax_tree: SyntaxTree<AstNode>,

    /// The set of semantic requirements declared in the source.
    requirements: HashSet<Requirement>,

    /// The symbol table built during semantic analysis.
    symbol_table: SymbolTable,

    /// String interner used for efficient symbol resolution.
    interner: StringInterner,

    /// The name of the source file or input from which the AST was parsed.
    source_name: String,

    /// Timestamp marking when semantic analysis was completed.
    generated_at: SystemTime,
}

impl Context {
    /// Creates a new semantic context from its components.
    ///
    /// # Parameters
    /// - `syntax_tree`: The arena-based AST nodes.
    /// - `source_name`: The source file or input name.
    /// - `requirements`: The set of semantic requirements extracted.
    /// - `symbol_table`: The symbol table constructed during analysis.
    /// - `interner`: The string interner instance.
    /// - `generated_at`: The timestamp marking the analysis time.
    ///
    /// # Returns
    /// A new `Context` instance.
    pub fn new(
        syntax_tree: SyntaxTree<AstNode>,
        source_name: String,
        requirements: HashSet<Requirement>,
        symbol_table: SymbolTable,
        interner: StringInterner,
        generated_at: SystemTime,
    ) -> Self {
        Self {
            syntax_tree,
            source_name,
            requirements,
            symbol_table,
            interner,
            generated_at,
        }
    }

    /// Extracts all semantic requirements from the syntax tree.
    ///
    /// This assumes all requirements are grouped under a single `RequireDef` node.
    ///
    /// # Arguments
    /// - `syntax_tree`: Reference to the arena-based syntax tree.
    ///
    /// # Returns
    /// A set of all declared and implied `Requirement`s.
    ///
    /// # Errors
    /// Returns `SemanticError` if traversing the tree fails.
    fn extract_requirements(syntax_tree: &SyntaxTree<AstNode>) -> Result<HashSet<Requirement>, SemanticError> {
        let mut requirements = HashSet::new();

        // Find the first RequireDef node
        let mut requirement_def_node = None;
        for node in syntax_tree.preorder().values() {
            if matches!(node.kind(), AstKind::RequireDef) {
                requirement_def_node = Some(node);
                break;
            }
        }

        if let Some(req_def) = requirement_def_node {
            // Collect all Requirement children
            for child in req_def.children() {
                let node = syntax_tree.try_node(*child)?;
                if matches!(node.kind(), AstKind::Requirement) {
                    if let Ok(req) = node.try_requirement() {
                        requirements.extend(req.imply());
                    }
                }
            }
        }

        Ok(requirements)
    }

    /// Checks if a given semantic requirement is declared.
    ///
    /// # Arguments
    /// - `requirement`: Reference to the `Requirement` to check.
    ///
    /// # Returns
    /// `true` if the requirement is declared, `false` otherwise.
    ///
    /// # Example
    /// ```rust
    /// if context.has_requirement(&Requirement::Fluent) {
    ///     println!("Fluent feature is enabled.");
    /// }
    /// ```
    pub fn has_requirement(&self, requirement: &Requirement) -> bool {
        self.requirements.contains(requirement)
    }

    /// Returns a reference to the AST node by its ID if it exists.
    pub fn get_node(&self, id: NodeId) -> Option<&AstNode> {
        self.syntax_tree.get_node(id)
    }

    /// Returns a reference to the AST node by its ID or an error if not found.
    pub fn try_node(&self, id: NodeId) -> Result<&AstNode, SemanticError> {
        Ok(self.syntax_tree.try_node(id)?)
    }

    /// Returns a reference to the full syntax tree.
    pub fn syntax_tree(&self) -> &SyntaxTree<AstNode> {
        &self.syntax_tree
    }

    /// Returns a mutable reference to the syntax tree.
    pub fn ast_mut(&mut self) -> &mut SyntaxTree<AstNode> {
        &mut self.syntax_tree
    }

    /// Takes ownership of the syntax tree, leaving an empty one in its place.
    pub fn take_syntax_tree(&mut self) -> SyntaxTree<AstNode> {
        std::mem::take(&mut self.syntax_tree)
    }

    /// Returns a reference to the set of semantic requirements.
    pub fn requirements(&self) -> &HashSet<Requirement> {
        &self.requirements
    }

    /// Returns a mutable reference to the set of semantic requirements.
    pub fn requirements_mut(&mut self) -> &mut HashSet<Requirement> {
        &mut self.requirements
    }

    /// Takes ownership of the requirements set, leaving it empty.
    pub fn take_requirements(&mut self) -> HashSet<Requirement> {
        std::mem::take(&mut self.requirements)
    }

    /// Returns a reference to the symbol table.
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Returns a mutable reference to the symbol table.
    pub fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    /// Takes ownership of the symbol table, leaving an empty one in its place.
    pub fn take_symbol_table(&mut self) -> SymbolTable {
        std::mem::take(&mut self.symbol_table)
    }

    /// Returns the timestamp when the semantic context was generated.
    pub fn generated_at(&self) -> SystemTime {
        self.generated_at
    }

    /// Returns the source file or input name associated with this context.
    pub fn source_name(&self) -> &String {
        &self.source_name
    }

    /// Returns a reference to the string interner.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Sets a new string interner instance.
    pub fn set_interner(&mut self, interner: StringInterner) {
        self.interner = interner;
    }
}

impl fmt::Display for Context {
    /// Formats the semantic context for human-readable output.
    ///
    /// The output includes:
    /// - Source name
    /// - Generation timestamp (seconds since UNIX epoch)
    /// - Declared requirements
    /// - Debug representation of the syntax tree
    /// - Display of the symbol table
    ///
    /// # Example output
    ///
    /// ```text
    /// Semantic Context Report:
    ///
    /// Source: domain.pddl
    /// Generated at: 1718523096 seconds since UNIX epoch
    ///
    /// Requirements:
    ///   - :strips
    ///   - :typing
    ///
    /// Abstract Syntax Tree:
    /// Node(kind=Domain, span=..., children=[...])
    ///
    /// Symbol Table:
    /// ...
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Semantic Context Report:\n")?;
        writeln!(f, "Source: {}", self.source_name)?;

        let duration_since_epoch = self.generated_at.duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::new(0, 0));
        writeln!(f, "Generated at: {} seconds since UNIX epoch\n", duration_since_epoch.as_secs())?;

        writeln!(f, "Requirements:")?;
        for req in &self.requirements {
            writeln!(f, "  - {}", req)?;
        }

        writeln!(f, "\nAbstract Syntax Tree:\n{:?}", self.syntax_tree)?;
        writeln!(f, "\nSymbol Table:\n{}", self.symbol_table)?;

        Ok(())
    }
}

impl TryFrom<&mut Ast> for Context {
    type Error = SemanticError;

    /// Attempts to create a semantic context by annotating a mutable AST reference.
    ///
    /// This process builds the symbol table, extracts semantic requirements,
    /// takes ownership of the AST arena and string interner, and sets the
    /// current time as the generation timestamp.
    ///
    /// # Arguments
    /// - `ast`: A mutable reference to the AST to annotate.
    ///
    /// # Returns
    /// - `Ok(Context)` if successful.
    /// - `Err(SemanticError)` if any semantic error occurs during annotation.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::convert::TryFrom;
    /// use aiplan4rust::semantic::Context;
    ///
    /// let mut ast = ...; // Previously parsed AST
    /// let context = Context::try_from(&mut ast)?;
    /// ```
    fn try_from(ast: &mut Ast) -> Result<Self, Self::Error> {
        let symbol_table = SymbolTable::try_from(&*ast)?;
        let syntax_tree = ast.take_syntax_tree();
        let requirements = Self::extract_requirements(&syntax_tree)?;
        let interner = ast.take_interner();

        Ok(Context::new(
            syntax_tree,
            ast.source_name().to_string(),
            requirements,
            symbol_table,
            interner,
            SystemTime::now(),
        ))
    }
}

impl SerdeSerializable for Context {}
