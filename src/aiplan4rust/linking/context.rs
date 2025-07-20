use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::arena::Arena;
use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::serialization::serde::SerdeSerializable;
use crate::aiplan4rust::syntax::core::SyntaxTree;

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
    domain_ast: SyntaxTree<AstNode>,
    problem_ast: SyntaxTree<AstNode>,
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
    /// * `domain` - The AST arena representing the domain context.
    /// * `problem` - The AST arena representing the problem context.
    /// * `symbol_table` - The combined symbol table after linking domain and problem.
    /// * `interner` - The unified string interner used for identifiers.
    /// * `domain_source` - The source (e.g. filename) of the domain.
    /// * `problem_source` - The source (e.g. filename) of the problem.
    ///
    /// # Returns
    ///
    /// A new `LinkedSemanticContext` instance with the current system time as the generation timestamp.
    pub fn new(
        domain_ast: SyntaxTree<AstNode>,
        problem_ast: SyntaxTree<AstNode>,
        domain_table: SymbolTable,
        problem_table: SymbolTable,
        interner: StringInterner,
        domain_source: String,
        problem_source: String,
    ) -> Self {
        LinkedSemanticContext {
            domain_ast,
            problem_ast,
            domain_table,
            problem_table,
            interner,
            domain_source,
            problem_source,
            generated_at: std::time::SystemTime::now(),
        }
    }

    /// Returns an immutable reference to the domain AST arena.
    pub fn domain_ast(&self) -> &SyntaxTree<AstNode> {
        &self.domain_ast
    }

    /// Returns a mutable reference to the domain AST arena.
    pub fn domain_ast_mut(&mut self) -> &mut SyntaxTree<AstNode> {
        &mut self.domain_ast
    }

    /// Returns an immutable reference to the problem AST arena.
    pub fn problem_ast(&self) -> &SyntaxTree<AstNode> {
        &self.problem_ast
    }

    /// Returns a mutable reference to the problem AST arena.
    pub fn problem_ast_mut(&mut self) -> &mut SyntaxTree<AstNode> {
        &mut self.problem_ast
    }


    /// Returns a reference to the domain symbol table.
    pub fn domain_table(&self) -> &SymbolTable {
        &self.domain_table
    }

    /// Returns a mutable reference to the domain symbol table.
    pub fn domain_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.domain_table
    }

    /// Returns a reference to the problem symbol table.
    pub fn problem_table(&self) -> &SymbolTable {
        &self.problem_table
    }

    /// Returns a mutable reference to the problem symbol table.
    pub fn problem_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.problem_table
    }

    /// Returns an immutable reference to the unified string interner.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Returns a mutable reference to the unified string interner.
    pub fn interner_mut(&mut self) -> &mut StringInterner {
        &mut self.interner
    }

    /// Returns a reference to the domain source (e.g. filename or identifier).
    pub fn domain_source(&self) -> &str {
        &self.domain_source
    }

    /// Returns a mutable reference to the domain source string.
    pub fn domain_source_mut(&mut self) -> &mut String {
        &mut self.domain_source
    }

    /// Returns a reference to the problem source (e.g. filename or identifier).
    pub fn problem_source(&self) -> &str {
        &self.problem_source
    }

    /// Returns a mutable reference to the problem source string.
    pub fn problem_source_mut(&mut self) -> &mut String {
        &mut self.problem_source
    }

    /// Returns the timestamp when this linked context was generated.
    pub fn generated_at(&self) -> std::time::SystemTime {
        self.generated_at
    }
}

impl fmt::Display for LinkedSemanticContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "LinkedSemanticContext Summary:")?;
        writeln!(f, "  Domain source: {}", self.domain_source)?;
        writeln!(f, "  Problem source: {}", self.problem_source)?;
        writeln!(f, "  Generated at: {:?}", self.generated_at)?;
        writeln!(f, "  Domain AST nodes:\n{}", self.domain_ast)?;
        writeln!(f, "  Domain symbol table entries:\n{}", self.domain_table)?;
        writeln!(f, "  Problem AST nodes:\n{}", self.problem_ast)?;
        writeln!(f, "  Problem symbol table entries:\n{}", self.problem_table)?;
        Ok(())
    }
}

impl SerdeSerializable for LinkedSemanticContext {}
