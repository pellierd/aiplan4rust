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

use crate::aiplan4rust::interner::{InternerError, Literal, StringInterner};
use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::serialization::serde::SerdeSerializable;
use crate::aiplan4rust::syntax::tree::SyntaxTree;

/// Represents a linked semantic context combining a domain and a problem.
///
/// This structure holds the merged and linked Abstract Syntax Trees (ASTs) for both
/// the domain and the problem, along with their respective symbol tables and a unified
/// string interner. It also stores metadata about the source files and the timestamp
/// when the linking was performed.
///
/// # Fields
///
/// * `domain_syntax_tree` - The AST arena representing the domain context.
/// * `problem_syntax_tree` - The AST arena representing the problem context after linking.
/// * `domain_table` - The symbol table for the domain context.
/// * `problem_table` - The symbol table for the problem context.
/// * `interner` - The unified string interner used for identifiers across domain and problem.
/// * `domain_source_id` - The identifier (literal) of the domain source file or module.
/// * `problem_source_id` - The identifier (literal) of the problem source file or module.
/// * `generated_at` - The timestamp when this linked context was created.
///
/// # Methods
///
/// This struct provides accessor methods for each field, both immutable and mutable:
///
/// - `domain()` / `domain_mut()`
/// - `problem()` / `problem_mut()`
/// - `domain_table()` / `domain_table_mut()`
/// - `problem_table()` / `problem_table_mut()`
/// - `interner()` / `interner_mut()`
/// - `domain_source_id()` / `domain_source_id_mut()`
/// - `problem_source_id()` / `problem_source_id_mut()`
/// - `generated_at()` / `generated_at_mut()`
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
    domain_source_id: Literal,
    problem_source_id: Literal,
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
    /// * `domain_source_id` - The literal identifier representing the domain source (e.g., filename or module).
    /// * `problem_source_id` - The literal identifier representing the problem source (e.g., filename or module).
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
        domain_source_id: Literal,
        problem_source_id: Literal,
    ) -> Self {
        LinkedSemanticContext {
            domain_syntax_tree,
            problem_syntax_tree,
            domain_table,
            problem_table,
            interner,
            domain_source_id,
            problem_source_id,
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

    /// Returns the domain source identifier.
    ///
    /// This identifier typically represents the source of the domain,
    /// such as a filename or module name, stored as a `Literal`.
    ///
    /// # Returns
    ///
    /// The `Literal` corresponding to the domain source.
    pub fn domain_source_id(&self) -> Literal {
        self.domain_source_id
    }

    /// Returns the problem source identifier.
    ///
    /// This identifier typically represents the source of the problem,
    /// such as a filename or module name, stored as a `Literal`.
    ///
    /// # Returns
    ///
    /// The `Literal` corresponding to the problem source.
    pub fn problem_source_id(&self) -> Literal {
        self.problem_source_id
    }

    /// Attempts to resolve and return the domain source name as a string slice from the interner.
    ///
    /// This method uses the `Literal` identifier returned by `domain_source_id()` to look up
    /// the actual source name string in the associated `StringInterner`.
    ///
    /// # Returns
    ///
    /// * `Ok(&str)` containing the resolved source name if successful.
    /// * `Err(InternerError)` if the `Literal` cannot be resolved, e.g., if the
    ///   source name is not set or invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// match ctx.try_domain_source_name() {
    ///     Ok(name) => println!("Domain source name: {}", name),
    ///     Err(_) => println!("Domain source name could not be resolved"),
    /// }
    /// ```
    pub fn try_domain_source_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_literal(self.domain_source_id())
    }

    /// Returns the domain source name as a string slice if it can be resolved from the interner.
    ///
    /// This method attempts to resolve the interned `Literal` representing the domain source
    /// (e.g., filename or origin) into a string slice by querying the associated `StringInterner`.
    ///
    /// # Returns
    ///
    /// * `Some(&str)` containing the domain source name if it exists in the interner.
    /// * `None` if the domain source name cannot be found or is not set.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Some(name) = ctx.domain_source_name() {
    ///     println!("Domain source name: {}", name);
    /// } else {
    ///     println!("Domain source name not available");
    /// }
    /// ```
    pub fn domain_source_name(&self) -> Option<&str> {
        self.interner.resolve_literal(self.domain_source_id())
    }

    /// Attempts to resolve and return the problem source name as a string slice from the interner.
    ///
    /// This method uses the `Literal` identifier returned by `problem_source_id()` to look up
    /// the actual source name string in the associated `StringInterner`.
    ///
    /// # Returns
    ///
    /// * `Ok(&str)` containing the resolved source name if successful.
    /// * `Err(InternerError)` if the `Literal` cannot be resolved, e.g., if the
    ///   source name is not set or invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// match ctx.try_problem_source_name() {
    ///     Ok(name) => println!("Problem source name: {}", name),
    ///     Err(_) => println!("Problem source name could not be resolved"),
    /// }
    /// ```
    pub fn try_problem_source_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_literal(self.problem_source_id())
    }

    /// Returns the domain source name as a `String`.
    ///
    /// Attempts to resolve the domain source `Literal` in the interner.
    /// If the literal cannot be resolved, returns `"Unknown<{:?}>"` where
    /// `{:?}` is the debug representation of the `Literal`.
    pub fn domain_source_name_string(&self) -> String {
        self.interner
            .resolve_literal(self.domain_source_id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("unknown<{}>", self.domain_source_id))
    }

    /// Returns the problem source name as a string slice if it can be resolved from the interner.
    ///
    /// This method attempts to resolve the interned `Literal` representing the problem source
    /// (e.g., filename or origin) into a string slice by querying the associated `StringInterner`.
    ///
    /// # Returns
    ///
    /// * `Some(&str)` containing the problem source name if it exists in the interner.
    /// * `None` if the problem source name cannot be found or is not set.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Some(name) = ctx.problem_source_name() {
    ///     println!("Problem source name: {}", name);
    /// } else {
    ///     println!("Problem source name not available");
    /// }
    /// ```
    pub fn problem_source_name(&self) -> Option<&str> {
        self.interner.resolve_literal(self.problem_source_id())
    }

    /// Returns the problem source name as a `String`.
    ///
    /// Attempts to resolve the problem source `Literal` in the interner.
    /// If the literal cannot be resolved, returns `"Unknown<{:?}>"` where
    /// `{:?}` is the debug representation of the `Literal`.
    pub fn problem_source_name_string(&self) -> String {
        self.interner
            .resolve_literal(self.problem_source_id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("unknown<{}>", self.problem_source_id))
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
        writeln!(f, "  Domain source: {}", self.domain_source_name_string())?;
        writeln!(f, "  Problem source: {}", self.problem_source_name_string())?;
        writeln!(f, "  Generated at: {:?}", self.generated_at)?;
        writeln!(f, "  Domain AST nodes:\n{}", self.domain_syntax_tree)?;
        writeln!(f, "  Domain symbol table entries:\n{}", self.domain_table)?;
        writeln!(f, "  Problem AST nodes:\n{}", self.problem_syntax_tree)?;
        writeln!(f, "  Problem symbol table entries:\n{}", self.problem_table)?;
        Ok(())
    }
}

impl SerdeSerializable for LinkedSemanticContext {}
