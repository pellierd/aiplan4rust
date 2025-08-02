//! This module defines the `LinkedSemanticContext` struct and associated functionality.
//!
//! The `LinkedSemanticContext` represents the result of linking the semantic information
//! of a domain and a problem in an AI syntax context.
//!
//! It holds the combined Abstract Syntax Trees (ASTs) for both the domain and the problem,
//! their corresponding symbol tables, and a unified string interner that manages identifiers
//! consistently across both contexts.
//!
//! Additionally, it stores metadata such as the source locations of the domain and problem,
//! as well as a timestamp indicating when the linking was performed.
//!
//! This module provides the core data structure for semantic linking and accessor methods
//! to retrieve or modify the linked semantic data.
//!
//! The module also implements serialization traits to support persistence or transmission,
//! and the `Display` trait for human-readable summaries of the linked context.
//!
use std::fmt;
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::serialization::serde::SerdeSerializable;
use crate::aiplan4rust::syntax::tree::SyntaxTree;

/// Represents a linked semantic context combining a domain and a problem.
///
/// This structure holds the merged and linked Abstract Syntax Trees (ASTs) for both
/// the domain and the problem, along with a combined symbol table and a unified string interner.
/// It also stores metadata about the source files and the timestamp when the linking was performed.
///
/// # Fields
///
/// * `domain` - The AST arena representing the domain context.
/// * `problem` - The AST arena representing the problem context after linking.
/// * `symbol_table` - The combined symbol table reflecting all linked symbols.
/// * `interner` - The unified string interner used for identifiers across domain and problem.
/// * `domain_source` - The source file or identifier for the domain.
/// * `problem_source` - The source file or identifier for the problem.
/// * `generated_at` - The timestamp when this linked context was created.
///
/// # Methods
///
/// This struct provides accessor methods for each field, both immutable and mutable:
///
/// * `domain()` / `domain_mut()`
/// * `problem()` / `problem_mut()`
/// * `symbol_table()` / `symbol_table_mut()`
/// * `interner()` / `interner_mut()`
/// * `domain_source()` / `domain_source_mut()`
/// * `problem_source()` / `problem_source_mut()`
/// * `generated_at()` / `generated_at_mut()`
///
/// # Display Implementation
///
/// Implements the `Display` trait to provide a human-readable summary of the linked semantic context,
/// including source information, timestamps, and counts of AST nodes and symbol table entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedSemanticContext {
    domain_syntax_tree: SyntaxTree<AstNode>,
    problem_syntax_tree: SyntaxTree<AstNode>,
    domain_table: SymbolTable,
    problem_table: SymbolTable,
    interner: StringInterner,
    domain_source: String,
    problem_source: String,
    generated_at: std::time::SystemTime,
}

impl LinkedSemanticContext {
    /// Creates a new `LinkedSemanticContext` from its components.
    ///
    /// # Arguments
    ///
    /// * `domain_syntax_tree` - The abstract syntax tree (AST) representing the domain context.
    /// * `problem_syntax_tree` - The abstract syntax tree (AST) representing the problem context.
    /// * `domain_table` - The symbol table for the domain.
    /// * `problem_table` - The symbol table for the problem.
    /// * `interner` - The unified string interner used for identifiers.
    /// * `domain_source` - The source (e.g., filename) of the domain.
    /// * `problem_source` - The source (e.g., filename) of the problem.
    ///
    /// # Returns
    ///
    /// A new instance of `LinkedSemanticContext` with the generation timestamp
    /// set to the current system time.
    pub fn new(
        domain_syntax_tree: SyntaxTree<AstNode>,
        problem_syntax_tree: SyntaxTree<AstNode>,
        domain_table: SymbolTable,
        problem_table: SymbolTable,
        interner: StringInterner,
        domain_source: String,
        problem_source: String,
    ) -> Self {
        LinkedSemanticContext {
            domain_syntax_tree,
            problem_syntax_tree,
            domain_table,
            problem_table,
            interner,
            domain_source,
            problem_source,
            generated_at: std::time::SystemTime::now(),
        }
    }

    /// Returns an immutable reference to the domain AST.
    ///
    /// # Returns
    ///
    /// A reference to the `SyntaxTree` representing the domain AST.
    pub fn domain_syntax_tree(&self) -> &SyntaxTree<AstNode> {
        &self.domain_syntax_tree
    }

    /// Returns a mutable reference to the domain AST.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `SyntaxTree` representing the domain AST.
    pub fn domain_syntax_tree_mut(&mut self) -> &mut SyntaxTree<AstNode> {
        &mut self.domain_syntax_tree
    }

    /// Returns an immutable reference to the problem AST.
    ///
    /// # Returns
    ///
    /// A reference to the `SyntaxTree` representing the problem AST.
    pub fn problem_syntax_tree(&self) -> &SyntaxTree<AstNode> {
        &self.problem_syntax_tree
    }

    /// Returns a mutable reference to the problem AST.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `SyntaxTree` representing the problem AST.
    pub fn problem_syntax_tree_mut(&mut self) -> &mut SyntaxTree<AstNode> {
        &mut self.problem_syntax_tree
    }

    /// Returns an immutable reference to the domain symbol table.
    ///
    /// # Returns
    ///
    /// A reference to the domain's `SymbolTable`.
    pub fn domain_table(&self) -> &SymbolTable {
        &self.domain_table
    }

    /// Returns a mutable reference to the domain symbol table.
    ///
    /// # Returns
    ///
    /// A mutable reference to the domain's `SymbolTable`.
    pub fn domain_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.domain_table
    }

    /// Returns an immutable reference to the problem symbol table.
    ///
    /// # Returns
    ///
    /// A reference to the problem's `SymbolTable`.
    pub fn problem_table(&self) -> &SymbolTable {
        &self.problem_table
    }

    /// Returns a mutable reference to the problem symbol table.
    ///
    /// # Returns
    ///
    /// A mutable reference to the problem's `SymbolTable`.
    pub fn problem_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.problem_table
    }

    /// Returns an immutable reference to the unified string interner.
    ///
    /// # Returns
    ///
    /// A reference to the `StringInterner` used for identifier management.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Returns a mutable reference to the unified string interner.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `StringInterner`.
    pub fn interner_mut(&mut self) -> &mut StringInterner {
        &mut self.interner
    }

    /// Takes ownership of the internal `StringInterner`, leaving a new empty one in its place.
    ///
    /// # Returns
    ///
    /// The previously held `StringInterner`.
    pub fn take_interner(&mut self) -> StringInterner {
        std::mem::take(&mut self.interner)
    }

    /// Returns a reference to the domain source identifier.
    ///
    /// # Returns
    ///
    /// A string slice representing the source (e.g., filename) of the domain.
    pub fn domain_source(&self) -> &str {
        &self.domain_source
    }

    /// Returns a mutable reference to the domain source string.
    ///
    /// # Returns
    ///
    /// A mutable reference to the domain source string.
    pub fn domain_source_mut(&mut self) -> &mut String {
        &mut self.domain_source
    }

    /// Returns a reference to the problem source identifier.
    ///
    /// # Returns
    ///
    /// A string slice representing the source (e.g., filename) of the problem.
    pub fn problem_source(&self) -> &str {
        &self.problem_source
    }

    /// Returns a mutable reference to the problem source string.
    ///
    /// # Returns
    ///
    /// A mutable reference to the problem source string.
    pub fn problem_source_mut(&mut self) -> &mut String {
        &mut self.problem_source
    }

    /// Returns the timestamp when this linked context was generated.
    ///
    /// # Returns
    ///
    /// A `SystemTime` representing when the `LinkedSemanticContext` was created.
    pub fn generated_at(&self) -> std::time::SystemTime {
        self.generated_at
    }
}

impl fmt::Display for LinkedSemanticContext {
    /// Formats the `LinkedSemanticContext` for user-friendly display.
    ///
    /// This implementation outputs a multi-line summary including:
    /// - Source file names for the domain and problem.
    /// - Timestamp of when the linked context was generated.
    /// - String representations of the domain and problem AST nodes.
    /// - Contents of the domain and problem symbol tables.
    ///
    /// # Arguments
    ///
    /// * `f` - A mutable reference to a `fmt::Formatter` used for writing the formatted output.
    ///
    /// # Returns
    ///
    /// Returns a `fmt::Result` indicating success or failure of the write operations.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the write operations fail.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "LinkedSemanticContext Summary:")?;
        writeln!(f, "  Domain source: {}", self.domain_source)?;
        writeln!(f, "  Problem source: {}", self.problem_source)?;
        writeln!(f, "  Generated at: {:?}", self.generated_at)?;
        writeln!(f, "  Domain AST nodes:\n{}", self.domain_syntax_tree)?;
        writeln!(f, "  Domain symbol table entries:\n{}", self.domain_table)?;
        writeln!(f, "  Problem AST nodes:\n{}", self.problem_syntax_tree)?;
        writeln!(f, "  Problem symbol table entries:\n{}", self.problem_table)?;
        Ok(())
    }
}

impl SerdeSerializable for LinkedSemanticContext {}
