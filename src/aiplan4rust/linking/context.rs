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
//! This module provides the common data structure for semantic linking and accessor methods
//! to retrieve or modify the linked semantic data.
//!
//! The module also implements serialization traits to support persistence or transmission,
//! and the `Display` trait for human-readable summaries of the linked context.
//!

use crate::aiplan4rust::cli::io::serialization::serde::SerdeSerializable;
use crate::aiplan4rust::core::interner::{InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{LiteralId, Requirement};
use crate::aiplan4rust::linking::LinkingError;
use crate::aiplan4rust::semantic::{SemanticContext, SymbolTable};
use crate::aiplan4rust::syntax::ast::tree::Tree;
use crate::aiplan4rust::syntax::ast::AstNode;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::time::SystemTime;

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
    domain_syntax_tree: Tree<AstNode>,
    problem_syntax_tree: Tree<AstNode>,
    domain_table: SymbolTable,
    problem_table: SymbolTable,
    declared_requirements: HashSet<Requirement>,
    required_requirements: HashSet<Requirement>,
    interner: SymbolInterner,
    domain_source_id: LiteralId,
    problem_source_id: LiteralId,
    generated_at: std::time::SystemTime,
}

impl Default for LinkedSemanticContext {
    /// Returns a debug `LinkedSemanticContext`.
    ///
    /// This debug context is primarily intended for initialization or placeholder purposes.
    /// All fields are set to their respective debug values, except for `generated_at`,
    /// which is initialized to the current system time (`SystemTime::now()`).
    ///
    /// # Fields Default Values
    /// - `domain_syntax_tree` and `problem_syntax_tree` – `Default::debug()`
    /// - `domain_table` and `problem_table` – `Default::debug()`
    /// - `declared_requirements` and `required_requirements` – empty `HashSet`
    /// - `interner` – empty `StringInterner`
    /// - `domain_source_id` and `problem_source_id` – `Default::debug()`
    /// - `generated_at` – current system time
    ///
    /// # Returns
    /// A `LinkedSemanticContext` with all fields initialized to debug values.
    fn default() -> Self {
        LinkedSemanticContext {
            domain_syntax_tree: Default::default(),
            problem_syntax_tree: Default::default(),
            domain_table: Default::default(),
            problem_table: Default::default(),
            declared_requirements: Default::default(),
            required_requirements: Default::default(),
            interner: Default::default(),
            domain_source_id: Default::default(),
            problem_source_id: Default::default(),
            generated_at: SystemTime::now(), // ou UNIX_EPOCH
        }
    }
}
impl LinkedSemanticContext {
    /// Creates a new `LinkedSemanticContext` by taking ownership of the domain and problem
    /// semantic contexts and merging their information into a single linked context.
    ///
    /// This constructor assumes that the `domain` and `problem` contexts have already
    /// been semantically linked or are ready to be linked, meaning that any external
    /// references in the problem can be resolved against the domain.
    ///
    /// The provided `interner` is a **global string interner** shared across both
    /// the domain and problem contexts, ensuring that identifiers and literals
    /// are consistent and unique throughout the linked context.
    ///
    /// # Arguments
    ///
    /// * `domain` - A mutable reference to the semantic context of the domain.
    ///              The domain AST and symbol table will be taken and included in
    ///              the linked context.
    /// * `problem` - A mutable reference to the semantic context of the problem.
    ///               The problem AST and symbol table will be taken and included in
    ///               the linked context.
    /// * `interner` - The unified `StringInterner` to use for both domain and problem,
    ///                ensuring consistent identifier resolution.
    ///
    /// # Behavior
    ///
    /// This function performs the following steps:
    /// 1. Takes ownership of the ASTs from both domain and problem contexts.
    /// 2. Verifies that the ASTs respect invariants (domain AST must be a domain, problem AST must be a problem, and hierarchical flags must be consistent).
    /// 3. Takes ownership of the symbol tables from both contexts.
    /// 4. Merges the `declared_requirements` and `required_requirements` from both contexts.
    /// 5. Takes the source IDs from both contexts.
    /// 6. Returns a fully constructed `LinkedSemanticContext` with all combined information.
    ///
    /// # Returns
    ///
    /// * `Ok(LinkedSemanticContext)` if all invariants are satisfied and the ASTs can be linked.
    /// * `Err(LinkingError)` if any invariants are violated, such as:
    ///   - Domain AST is empty or not a valid domain.
    ///   - Problem AST is empty or not a valid problem.
    ///   - Hierarchical flags mismatch between domain and problem.
    ///
    /// # Note
    ///
    /// After calling this function, the `domain` and `problem` contexts will have
    /// their ASTs and symbol tables taken, so they should not be used further unless
    /// reconstructed or cloned.
    pub fn new(
        mut domain: SemanticContext,
        mut problem: SemanticContext,
        interner: SymbolInterner,
    ) -> Result<Self, LinkingError> {
        // Verify invariants before constructing
        Self::check_invariant(&domain, &problem)?;

        // Take ASTs
        let domain_syntax_tree = domain.take_syntax_tree();
        let problem_syntax_tree = problem.take_syntax_tree();

        // Take symbol tables
        let domain_table = domain.take_symbol_table();
        let problem_table = problem.take_symbol_table();

        // Merge declared and required requirements
        let declared_requirements = domain
            .declared_requirements()
            .union(problem.declared_requirements())
            .cloned()
            .collect();

        let required_requirements: HashSet<Requirement> = domain
            .inferred_requirements()
            .expect("Domain analysis must be completed before linking")
            .union(
                problem
                    .inferred_requirements()
                    .expect("Problem analysis must be completed before linking"),
            )
            .cloned()
            .collect();

        // Step 5: Take interner and source ids
        let domain_source_id = domain.source();
        let problem_source_id = problem.source();

        Ok(LinkedSemanticContext {
            domain_syntax_tree,
            problem_syntax_tree,
            domain_table,
            problem_table,
            declared_requirements,
            required_requirements,
            interner,
            domain_source_id,
            problem_source_id,
            generated_at: SystemTime::now(),
        })
    }

    /// Checks that the domain and problem contexts respect the expected invariants:
    /// - The domain AST is not empty and its root kind corresponds to a domain.
    /// - The problem AST is not empty and its root kind corresponds to a problem.
    /// - The hierarchical requirement (`Requirement::Hierarchy`) is consistent between domain and problem.
    ///
    /// # Arguments
    ///
    /// * `domain` - The `SemanticContext` representing the domain.
    /// * `problem` - The `SemanticContext` representing the problem.
    ///
    /// # Errors
    ///
    /// Returns a `LinkingError` variant if any invariant is violated:
    /// - `EmptySyntaxTree` if the AST is empty.
    /// - `NotADomainSyntaxTree` if the domain AST root is not a domain.
    /// - `NotAProblemSyntaxTree` if the problem AST root is not a problem.
    /// - `HierarchicalMismatch` if the hierarchical requirement differs between domain and problem.
    ///
    /// # Notes
    ///
    /// This function now determines whether the domain and problem are hierarchical by checking
    /// the semantic requirements (`Requirement::Hierarchy`) rather than relying solely on the AST root kind.
    /// This ensures consistency with the declared semantic requirements in each context.
    pub fn check_invariant(
        domain: &SemanticContext,
        problem: &SemanticContext,
    ) -> Result<(), LinkingError> {
        // Check domain is non-empty and is a domain
        if domain.syntax_tree().is_empty() {
            return Err(LinkingError::empty_syntax_tree());
        }
        if !domain.is_domain() {
            return Err(LinkingError::not_a_domain_syntax_tree());
        }

        // Check problem is non-empty and is a problem
        if problem.syntax_tree().is_empty() {
            return Err(LinkingError::empty_syntax_tree());
        }
        if !problem.is_problem() {
            return Err(LinkingError::not_a_problem_syntax_tree());
        }

        // Check hierarchical consistency
        if domain.is_inferred(Requirement::Hierarchy) != problem.is_inferred(Requirement::Hierarchy)
        {
            return Err(LinkingError::hierarchical_mismatch());
        }

        Ok(())
    }

    /// Checks if a given semantic requirement is declared in the linked context.
    ///
    /// Declared requirements are those that are explicitly specified in either the
    /// domain or problem definitions. This does **not** guarantee that the requirement
    /// is actually used or needed by the AST content, only that it has been declared.
    ///
    /// # Arguments
    /// * `requirement` - The semantic requirement to check.
    ///
    /// # Returns
    /// `true` if the requirement is declared in the linked context, `false` otherwise.
    ///
    /// # Example
    /// ```rust
    /// if context.is_declared(Requirement::Fluents) {
    ///     println!("Fluents requirement is declared in this linked context.");
    /// }
    /// ```
    pub fn is_declared(&self, requirement: Requirement) -> bool {
        self.declared_requirements.contains(&requirement)
    }

    /// Checks if a given semantic requirement is actually required by the content of
    /// the linked ASTs (domain and problem).
    ///
    /// Required requirements represent the minimal set of features that are actually
    /// used in the domain or problem definitions. A requirement may be declared but
    /// not required if it is never referenced in the ASTs.
    ///
    /// # Arguments
    /// * `requirement` - The semantic requirement to check.
    ///
    /// # Returns
    /// `true` if the requirement is required by the linked ASTs, `false` otherwise.
    ///
    /// # Example
    /// ```rust
    /// if context.is_required(Requirement::DurativeActions) {
    ///     println!("DurativeActions are required by this linked context.");
    /// }
    /// ```
    pub fn is_required(&self, requirement: Requirement) -> bool {
        self.required_requirements.contains(&requirement)
    }

    /// Consumes and returns all declared requirements from the context.
    ///
    /// After calling this method, `declared_requirements` will be empty.
    ///
    /// # Example
    /// ```rust
    /// let declared = context.take_declared_requirements();
    /// println!("Declared requirements: {:?}", declared);
    /// ```
    pub fn take_declared_requirements(&mut self) -> HashSet<Requirement> {
        std::mem::take(&mut self.declared_requirements)
    }

    /// Consumes and returns all actually required requirements from the context.
    ///
    /// After calling this method, `required_requirements` will be empty.
    ///
    /// # Example
    /// ```rust
    /// let required = context.take_required_requirements();
    /// println!("Required requirements: {:?}", required);
    /// ```
    pub fn take_required_requirements(&mut self) -> HashSet<Requirement> {
        std::mem::take(&mut self.required_requirements)
    }

    /// Returns an immutable reference to the domain AST.
    ///
    /// # Returns
    ///
    /// A reference to the `SyntaxTree` representing the domain AST.
    pub fn domain_syntax_tree(&self) -> &Tree<AstNode> {
        &self.domain_syntax_tree
    }

    /// Returns a mutable reference to the domain AST.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `Tree<AstNode>` representing the domain AST.
    pub fn domain_syntax_tree_mut(&mut self) -> &mut Tree<AstNode> {
        &mut self.domain_syntax_tree
    }

    /// Takes ownership of the domain AST, leaving an empty AST in its place.
    ///
    /// # Returns
    ///
    /// The previously held `SyntaxTree` representing the domain.
    pub fn take_domain_syntax_tree(&mut self) -> Tree<AstNode> {
        std::mem::take(&mut self.domain_syntax_tree)
    }

    /// Returns an immutable reference to the problem AST.
    ///
    /// # Returns
    ///
    /// A reference to the `SyntaxTree` representing the problem AST.
    pub fn problem_syntax_tree(&self) -> &Tree<AstNode> {
        &self.problem_syntax_tree
    }

    /// Returns a mutable reference to the problem AST.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `Tree<AstNode>` representing the problem AST.
    pub fn problem_syntax_tree_mut(&mut self) -> &mut Tree<AstNode> {
        &mut self.problem_syntax_tree
    }

    /// Takes ownership of the problem AST, leaving an empty AST in its place.
    ///
    /// # Returns
    ///
    /// The previously held `SyntaxTree` representing the problem.
    pub fn take_problem_syntax_tree(&mut self) -> Tree<AstNode> {
        std::mem::take(&mut self.problem_syntax_tree)
    }

    /// Returns an immutable reference to the domain symbol table.
    ///
    /// # Returns
    ///
    /// A reference to the domain's `SymbolTable`.
    pub fn domain_table(&self) -> &SymbolTable {
        &self.domain_table
    }

    /// Takes ownership of the domain symbol table, leaving an empty table in its place.
    ///
    /// # Returns
    ///
    /// The previously held `SymbolTable` for the domain.
    pub fn take_domain_table(&mut self) -> SymbolTable {
        std::mem::take(&mut self.domain_table)
    }

    /// Returns an immutable reference to the problem symbol table.
    ///
    /// # Returns
    ///
    /// A reference to the problem's `SymbolTable`.
    pub fn problem_table(&self) -> &SymbolTable {
        &self.problem_table
    }

    /// Takes ownership of the problem symbol table, leaving an empty table in its place.
    ///
    /// # Returns
    ///
    /// The previously held `SymbolTable` for the problem.
    pub fn take_problem_table(&mut self) -> SymbolTable {
        std::mem::take(&mut self.problem_table)
    }

    /// Returns an immutable reference to the unified string interner.
    ///
    /// # Returns
    ///
    /// A reference to the `StringInterner` used for identifier management.
    pub fn interner(&self) -> &SymbolInterner {
        &self.interner
    }

    /// Takes ownership of the internal `StringInterner`, leaving a new empty one in its place.
    ///
    /// # Returns
    ///
    /// The previously held `StringInterner`.
    pub fn take_interner(&mut self) -> SymbolInterner {
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
    pub fn domain_source_id(&self) -> LiteralId {
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
    pub fn problem_source_id(&self) -> LiteralId {
        self.problem_source_id
    }

    /// Attempts to resolve a `Literal` into a string slice using the interner.
    ///
    /// This internal helper function centralizes the lookup ops for any `Literal` identifier.
    ///
    /// # Arguments
    ///
    /// * `lit` - The literal identifier to resolve.
    ///
    /// # Returns
    ///
    /// * `Some(&str)` if the literal exists in the interner.
    /// * `None` if the literal cannot be resolved.
    ///
    /// # Example
    ///
    /// ```rust
    /// if let Some(name) = ctx.source_name(literal) {
    ///     println!("Resolved name: {}", name);
    /// }
    /// ```
    fn source_name(&self, lit: LiteralId) -> Option<&str> {
        self.interner.resolve_literal(lit)
    }

    /// Returns a `String` representation of a `Literal`.
    ///
    /// Falls back to `"unknown<Literal>"` if the literal cannot be resolved in the interner.
    ///
    /// # Arguments
    ///
    /// * `lit` - The literal identifier to resolve.
    ///
    /// # Returns
    ///
    /// The resolved name as a `String`, or a placeholder if unresolved.
    ///
    /// # Example
    ///
    /// ```rust
    /// let name = ctx.source_name_string(literal);
    /// println!("Source name: {}", name);
    /// ```
    fn source_name_string(&self, lit: LiteralId) -> String {
        self.source_name(lit)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("unknown<{}>", lit))
    }

    /// Returns the domain source name as an `Option<&str>`.
    ///
    /// # Returns
    ///
    /// * `Some(&str)` if the domain source literal can be resolved.
    /// * `None` otherwise.
    pub fn domain_source_name(&self) -> Option<&str> {
        self.source_name(self.domain_source_id)
    }

    /// Returns the domain source name as a `String`.
    ///
    /// Falls back to `"unknown<Literal>"` if unresolved.
    pub fn domain_source_name_string(&self) -> String {
        self.source_name_string(self.domain_source_id)
    }

    /// Attempts to resolve the domain source name, returning a `Result`.
    ///
    /// # Returns
    ///
    /// * `Ok(&str)` if the domain source literal is resolved.
    /// * `Err(InternerError)` if the literal cannot be resolved.
    pub fn try_domain_source_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_literal(self.domain_source_id)
    }

    /// Returns the problem source name as an `Option<&str>`.
    ///
    /// # Returns
    ///
    /// * `Some(&str)` if the problem source literal can be resolved.
    /// * `None` otherwise.
    pub fn problem_source_name(&self) -> Option<&str> {
        self.source_name(self.problem_source_id)
    }

    /// Returns the problem source name as a `String`.
    ///
    /// Falls back to `"unknown<Literal>"` if unresolved.
    pub fn problem_source_name_string(&self) -> String {
        self.source_name_string(self.problem_source_id)
    }

    /// Attempts to resolve the problem source name, returning a `Result`.
    ///
    /// # Returns
    ///
    /// * `Ok(&str)` if the problem source literal is resolved.
    /// * `Err(InternerError)` if the literal cannot be resolved.
    pub fn try_problem_source_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_literal(self.problem_source_id)
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
    /// Formats the `LinkedSemanticContext` for human-readable display.
    ///
    /// This implementation outputs a structured, multi-line summary of the linked context,
    /// including domain and problem sources, generation timestamp, semantic requirements,
    /// AST contents, and symbol tables.
    ///
    /// # Details
    ///
    /// The summary includes:
    /// - **Domain source name** resolved from the interner, with fallback `"unknown<Literal>"`.
    /// - **Problem source name** resolved similarly.
    /// - **Generated at** timestamp, formatted in local date/time as `YYYY-MM-DD HH:MM:SS`.
    /// - **Declared requirements** listed in sorted order for clarity.
    /// - **Required requirements** listed in sorted order.
    /// - **Domain AST nodes** as a string representation of the syntax tree.
    /// - **Domain symbol table entries** as a string representation of the symbol table.
    /// - **Problem AST nodes** and **symbol table entries** similarly.
    ///
    /// # Arguments
    ///
    /// * `f` - A mutable reference to a `fmt::Formatter` used for writing the formatted output.
    ///
    /// # Returns
    ///
    /// * `fmt::Result` indicating success or failure of the write operations.
    ///
    /// # Notes
    ///
    /// - Requirements are sorted to provide a deterministic, human-readable output.
    /// - The timestamp uses `chrono::Local` to format `SystemTime` into a readable local date/time.
    /// - The domain and problem source names use the factorized `source_name_string` methods,
    ///   ensuring consistent fallback formatting if the literals cannot be resolved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::fmt;
    ///
    /// let ctx: LinkedSemanticContext = ...;
    /// println!("{}", ctx); // Uses the Display impl to print the summary.
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "LinkedSemanticContext Summary:")?;
        writeln!(f, "  Domain source: {}", self.domain_source_name_string())?;
        writeln!(f, "  Problem source: {}", self.problem_source_name_string())?;

        // Format SystemTime in local date/time
        let datetime: DateTime<Local> = self.generated_at.into();
        writeln!(
            f,
            "  Generated at: {}",
            datetime.format("%Y-%m-%d %H:%M:%S")
        )?;

        // Sorted declared requirements
        writeln!(f, "  Declared requirements:")?;
        let mut declared: Vec<_> = self.declared_requirements.iter().collect();
        declared.sort();
        for req in declared {
            writeln!(f, "    - {}", req)?;
        }

        // Sorted required requirements
        writeln!(f, "  Required requirements:")?;
        let mut required: Vec<_> = self.required_requirements.iter().collect();
        required.sort();
        for req in required {
            writeln!(f, "    - {}", req)?;
        }

        writeln!(f, "  Domain AST nodes:\n{}", self.domain_syntax_tree)?;
        writeln!(f, "  Domain symbol table entries:\n{}", self.domain_table)?;
        writeln!(f, "  Problem AST nodes:\n{}", self.problem_syntax_tree)?;
        writeln!(f, "  Problem symbol table entries:\n{}", self.problem_table)?;
        Ok(())
    }
}

impl SerdeSerializable for LinkedSemanticContext {}
