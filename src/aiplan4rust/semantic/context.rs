//! Semantic Analysis Module
//!
//! This module provides the common components for semantic analysis within the compiler pipeline.
//! It centralizes semantic-related data structures, error handling, and processing ops.
//!
//! # Overview
//!
//! The semantic analysis phase verifies the correctness of the syntax tree beyond parsing,
//! including symbol resolution, semantic checks, and typing checking. This module defines:
//!
//! - **`Context`**: The main semantic context structure holding the semantically enriched AST,
//!   symbol table, semantic requirements, and associated metadata.
//! - **Error Types**: Semantic-related error enumerations and specific error types such as
//!   `SemanticError`, `UnexpectedNodeKindError`, and `InvalidNodeArityError`.
//! - **Symbol Table Management**: Structures and utilities for symbol resolution during semantic analysis.
//! - **Analyzer Components**: Components that perform semantic checking and typing checking.
//!
//! # Submodules
//!
//! - `analyzer`: Implements the common semantic analyzer ops.
//! - `symbol`: Contains symbol representations used in semantic processing.
//! - `analyzer_result`: Defines result types used by analyzers.
//! - `symbol_table`: Manages the symbol table data structures.
//! - `checks`: Implements various semantic checks.
//! - `context`: Defines the `Context` structure and its associated functionality.
//! - `error`: Contains semantic error definitions and error handling utilities.
//! - `type_checker`: Handles typing checking ops and related operations.
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
//! The module uses `SemanticError` as a unified error typing that encompasses
//! errors from various semantic subcomponents such as symbol table errors,
//! typing checking errors, and unexpected AST node errors.
//!
//! This design enables streamlined error propagation and reporting during
//! semantic analysis.

use crate::aiplan4rust::cli::io::serialization::serde::SerdeSerializable;
use crate::aiplan4rust::semantic::{SemanticError, SymbolTable};
use crate::aiplan4rust::support::interner::{InternerError, SymbolInterner};
use crate::aiplan4rust::support::lang::{LiteralId, RemapSymbol, Requirement, SymbolId};
use crate::aiplan4rust::syntax::ast::tree::{NodeId, Tree};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};

use crate::aiplan4rust::linking::finalization::FinalizationContext;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::passes::PassContext;
use crate::aiplan4rust::support::diagnostic::Provider;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::SystemTime;

/// Holds the results of semantic analysis, including the syntax tree, symbol table,
/// requirements analysis, and associated metadata.
///
/// This structure represents the full context of a parsed and analyzed PDDL/HDDL file.
/// It acts as a standalone "semantic unit" encapsulating all data needed for further
/// processing stages such as linkage, validation, or code generation.
///
/// # Metadata and Requirements
///
/// The context distinguishes between **declared** requirements (what the user stated)
/// and **inferred** requirements (what the syntax actually uses). This duality allows
/// the compiler to detect missing declarations and provide precise diagnostics via
/// the `requirement_triggers` mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// The annotated syntax tree stored as an arena of AST nodes.
    /// This tree is the structural backbone of the context.
    syntax_tree: Tree<AstNode>,

    /// Requirements explicitly stated in the `:requirements` section of the PDDL file.
    /// Used to verify user intent against actual feature usage.
    declared_requirements: HashSet<Requirement>,

    /// The set of PDDL/HDDL features actually utilized in the syntax tree.
    ///
    /// This is a derived set from the keys of `requirement_triggers`, cached here
    /// for O(1) membership checks during linking and validation.
    inferred_requirements: Option<HashSet<Requirement>>,

    /// A detailed mapping between a required feature and the specific AST nodes
    /// that triggered its necessity.
    ///
    /// This map is crucial for the Linker to generate precise diagnostics,
    /// pointing exactly to the location in the source file that requires an
    /// undeclared requirement.
    requirement_triggers: Option<HashMap<Requirement, Vec<NodeId>>>,

    /// The symbol table built during semantic analysis.
    /// Contains all declarations (types, constants, predicates, etc.) found in the file.
    symbol_table: SymbolTable,

    /// String interner used for efficient identifier storage and comparison.
    /// This interner is local to the context until unified by a Linker.
    interner: SymbolInterner,

    /// The interned identifier of the source file or input (e.g., file path or buffer name).
    /// This avoids string duplication and allows for efficient cross-referencing.
    source: LiteralId,

    /// Timestamp marking when the semantic analysis was completed.
    /// Useful for caching strategies and invalidated build detection.
    generated_at: SystemTime,
}
impl Default for Context {
    /// Returns a debug `Context`.
    ///
    /// All fields are initialized to their respective defaults, except for `generated_at`,
    /// which is set to the current system time (`SystemTime::now()`).
    /// Creates a debug, empty `Context`.
    ///
    /// Note: `generated_at` is initialized to the current system time,
    /// representing the moment this empty context was instantiated.
    fn default() -> Self {
        Self {
            syntax_tree: Tree::default(),
            declared_requirements: HashSet::default(),
            inferred_requirements: None,
            requirement_triggers: None,
            symbol_table: SymbolTable::default(),
            interner: SymbolInterner::default(),
            source: LiteralId::default(),
            generated_at: SystemTime::now(),
        }
    }
}

impl Context {
    /// Creates a new semantic context from its pre-analyzed components and validates
    /// syntax tree invariants.
    ///
    /// Unlike previous versions, this constructor does not perform extraction itself;
    /// it receives the already processed symbol table and the rich requirement trigger
    /// map from the analysis pipeline.
    ///
    /// # Parameters
    /// - `syntax_tree`: The syntax tree representing the domain or problem AST.
    /// - `source_id`: Identifier of the source file or input from which this context is derived.
    /// - `symbol_table`: The fully resolved symbol table.
    /// - `interner`: The string interner used for symbol storage.
    /// - `declared_requirements`: Requirements explicitly stated in the `:requirements` section.
    /// - `requirement_triggers`: A mapping of requirements effectively used in the AST to
    ///   their triggering `NodeId`s.
    ///
    /// # Returns
    /// A `Result` containing the new `Context` instance, or a `SemanticError`
    /// if invariants are violated.
    ///
    /// # Logic
    /// - The `inferred_requirements` set is automatically derived from the keys of
    ///   `requirement_triggers` to ensure data consistency.
    /// - The `generated_at` timestamp is automatically set to the current system time.
    ///
    /// # Safety and Invariants
    /// In debug builds, this function calls `check_invariant` to ensure the tree root
    /// is a valid PDDL/HDDL domain or problem.
    pub(crate) fn new(
        syntax_tree: Tree<AstNode>,
        source_id: LiteralId,
        symbol_table: SymbolTable,
        interner: SymbolInterner,
        declared_requirements: HashSet<Requirement>,
        requirement_triggers: Option<HashMap<Requirement, Vec<NodeId>>>,
    ) -> Result<Self, SemanticError> {
        // Validate that the syntax tree is not empty and the root is domain/problem
        #[cfg(debug_assertions)]
        Self::check_invariant(&syntax_tree)?;

        // Derive the inferred requirements set from the trigger map keys
        let inferred_requirements = requirement_triggers
            .as_ref()
            .map(|triggers| triggers.keys().cloned().collect::<HashSet<Requirement>>());

        Ok(Self {
            syntax_tree,
            source: source_id,
            symbol_table,
            interner,
            declared_requirements,
            inferred_requirements,
            requirement_triggers,
            generated_at: SystemTime::now(),
        })
    }

    /// Checks that a syntax tree satisfies the required invariants:
    /// - The syntax tree is not empty.
    /// - The root node is a domain or problem: `Domain`, `Problem`.
    ///
    /// # Arguments
    /// - `tree`: The `SyntaxTree` to validate.
    ///
    /// # Errors
    /// - Returns `SemanticError::empty_syntax_tree()` if the tree is empty.
    /// - Returns `SemanticError::unexpected_syntax_tree_root()` if the root node is not a valid domain or problem.
    #[cfg(debug_assertions)]
    fn check_invariant(tree: &Tree<AstNode>) -> Result<(), SemanticError> {
        // Check that the tree is not empty
        let root_node = match tree.root_node() {
            Some(root) => root,
            None => return Err(SemanticError::empty_syntax_tree()),
        };

        // Check that the root node is a valid domain or problem
        match root_node.kind() {
            AstKind::Domain | AstKind::Problem => Ok(()),
            _ => Err(SemanticError::unexpected_syntax_tree_root()),
        }
    }

    /// Returns `true` if the syntax tree in this context is empty.
    ///
    /// An AST is considered empty if it has no root node.
    pub fn is_empty(&self) -> bool {
        self.syntax_tree.is_empty()
    }

    /// Returns `true` if this context represents a **domain** AST.
    ///
    /// Checks the `AstKind` of the root node and returns `true` if it is `Domain`.
    ///
    /// Returns `false` if the AST is empty or the root is a problem.
    pub fn is_domain(&self) -> bool {
        matches!(
            self.syntax_tree.root_node().map(|root| root.kind()),
            Some(AstKind::Domain)
        )
    }

    /// Returns `true` if this context represents a **problem** AST.
    ///
    /// Checks the `AstKind` of the root node and returns `true` if it is `Problem`.
    ///
    /// Returns `false` if the AST is empty or the root is a domain.
    pub fn is_problem(&self) -> bool {
        matches!(
            self.syntax_tree.root_node().map(|root| root.kind()),
            Some(AstKind::Problem)
        )
    }

    /// Checks if a given semantic requirement is declared in the context.
    ///
    /// # Arguments
    /// * `requirement` - The semantic requirement to check.
    ///
    /// # Returns
    /// `true` if the requirement is declared, `false` otherwise.
    ///
    /// # Example
    /// ```
    /// if context.is_declared(Requirement::Fluents) {
    ///     println!("Fluents requirement is declared in this context.");
    /// }
    /// ```
    pub fn is_declared(&self, requirement: Requirement) -> bool {
        self.declared_requirements.contains(&requirement)
    }

    /// Checks if a specific requirement has been inferred from the syntax tree.
    ///
    /// # Note
    /// This method returns `false` if the analysis has not been performed yet
    /// (`None` state), as no requirements have been officially detected.
    pub fn is_inferred(&self, requirement: Requirement) -> bool {
        self.inferred_requirements
            .as_ref()
            .map_or(false, |reqs| reqs.contains(&requirement))
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
    pub fn syntax_tree(&self) -> &Tree<AstNode> {
        &self.syntax_tree
    }

    /// Updates the context's syntax tree with a new version.
    ///
    /// # Warning
    /// Replacing the syntax tree is a structural change. If this context has already
    /// been analyzed, the existing `symbol_table` and `inferred_requirements` may
    /// become out of sync with the new tree structure. This is typically used by the
    /// Linker during AST merging or by optimization passes.
    ///
    /// # Parameters
    /// - `tree`: The new [`Tree<AstNode>`] to be used as the structural backbone
    ///   of this context.
    pub fn set_syntax_tree(&mut self, tree: Tree<AstNode>) {
        self.syntax_tree = tree;
    }

    /// Returns a mutable reference to the syntax tree.
    pub fn syntax_tree_mut(&mut self) -> &mut Tree<AstNode> {
        &mut self.syntax_tree
    }

    /// Takes ownership of the syntax tree, leaving an empty one in its place.
    pub fn take_syntax_tree(&mut self) -> Tree<AstNode> {
        std::mem::take(&mut self.syntax_tree)
    }

    /// Returns a reference to the set of declared semantic requirements.
    pub fn declared_requirements(&self) -> &HashSet<Requirement> {
        &self.declared_requirements
    }

    /// Returns the set of requirements inferred from the syntax tree.
    ///
    /// Returns `Some(&HashSet<Requirement>)` if the semantic analysis (inference pass)
    /// has been performed. Returns `None` if the inference pass is pending,
    /// which typically occurs for PDDL problems before they are linked to a domain.
    pub fn inferred_requirements(&self) -> Option<&HashSet<Requirement>> {
        self.inferred_requirements.as_ref()
    }

    /// Returns the mapping of requirements to their triggering AST nodes.
    ///
    /// Returns `Some(&HashMap<Requirement, Vec<NodeId>>)` if the analysis has
    /// been completed. This map provides the exact locations (NodeIds) that
    /// necessitate specific PDDL/HDDL features.
    ///
    /// Returns `None` if the analysis has not yet been executed.
    pub fn requirement_triggers(&self) -> Option<&HashMap<Requirement, Vec<NodeId>>> {
        self.requirement_triggers.as_ref()
    }

    /// Retrieves the specific AST nodes that triggered a given requirement.
    ///
    /// This is a convenience method that safely navigates the optional triggers map.
    ///
    /// # Returns
    /// - `Some(&[NodeId])`: A slice of node identifiers that utilize the requirement.
    /// - `None`: If the requirement is not present OR if the inference analysis
    ///   has not been performed yet.
    pub fn get_triggers_for(&self, requirement: &Requirement) -> Option<&[NodeId]> {
        self.requirement_triggers
            .as_ref()
            .and_then(|map| map.get(requirement))
            .map(|v| v.as_slice())
    }

    /// Sets the requirement triggers and synchronizes the inferred requirements set.
    ///
    /// This method transitions the context from an "uninitialized" or "pending" state
    /// to an "analyzed" state. It automatically populates `inferred_requirements`
    /// by collecting the keys from the provided triggers map, ensuring data consistency
    /// between the detailed trigger locations and the high-level requirement set.
    ///
    /// # Parameters
    /// - `triggers`: A map linking each utilized PDDL/HDDL [`Requirement`] to the
    ///   specific [`NodeId`]s that triggered its necessity.
    pub fn set_requirement_triggers(&mut self, triggers: HashMap<Requirement, Vec<NodeId>>) {
        // Derive the inferred requirements set from the triggers' keys and wrap in Some
        self.inferred_requirements = Some(triggers.keys().cloned().collect());

        // Store the triggers map and wrap in Some to mark the analysis as completed
        self.requirement_triggers = Some(triggers);
    }

    /// Returns a reference to the symbol table.
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Returns a mutable reference to the symbol table.
    pub fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    /// Définit une nouvelle table des symboles pour le contexte.
    ///
    /// Cette méthode remplace l'ancienne table par celle fournie en argument.
    /// Elle est généralement utilisée après une passe de simplification ou
    /// lors de l'initialisation du contexte sémantique.
    ///
    /// # Arguments
    ///
    /// * `table` - La nouvelle instance de [`SymbolTable`] à associer à ce contexte.
    pub fn set_symbol_table(&mut self, table: SymbolTable) {
        self.symbol_table = table;
    }

    /// Takes ownership of the symbol table, leaving an empty one in its place.
    pub fn take_symbol_table(&mut self) -> SymbolTable {
        std::mem::take(&mut self.symbol_table)
    }

    /// Returns the timestamp when the semantic context was generated.
    pub fn generated_at(&self) -> SystemTime {
        self.generated_at
    }
    /// Returns the interned identifier (`Literal`) for the source.
    ///
    /// This `Literal` acts as a key into the string interner to retrieve the actual
    /// source name (e.g., a filename like `"domain.pddl"`).
    ///
    /// # Returns
    ///
    /// The `Literal` representing the interned source name.
    pub fn source(&self) -> LiteralId {
        self.source
    }

    /// Returns the resolved source name as an `Option<&str>`.
    ///
    /// Looks up the interned `Literal` in the associated interner and returns the
    /// corresponding string slice if it exists.
    ///
    /// # Returns
    ///
    /// * `Some(&str)` if the source name is found.
    /// * `None` if the source name is not present in the interner.
    pub fn source_name(&self) -> Option<&str> {
        self.interner.resolve_literal(self.source)
    }

    /// Returns the resolved source name as a `String`.
    ///
    /// If the source name cannot be resolved, returns a fallback string in the format:
    /// `"Unknown<{:?}>"`, where the debug representation of the literal is included.
    ///
    /// # Returns
    ///
    /// A `String` representing the source name or a fallback placeholder.
    pub fn source_name_string(&self) -> String {
        self.source_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Unknown<{:?}>", self.source))
    }

    /// Attempts to resolve the source name as a string slice.
    ///
    /// Similar to [`source_name`], but returns a `Result` that can propagate
    /// interner-specific errors when the lookup fails.
    ///
    /// # Returns
    ///
    /// * `Ok(&str)` containing the source name if successfully resolved.
    /// * `Err(InternerError)` if the resolution fails.
    ///
    /// # Example
    ///
    /// ```
    /// match context.try_source_name() {
    ///     Ok(name) => println!("Source name: {}", name),
    ///     Err(e) => eprintln!("Failed to resolve source name: {:?}", e),
    /// }
    /// ```
    pub fn try_source_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_literal(self.source)
    }

    /// Returns a reference to the string interner.
    pub fn interner(&self) -> &SymbolInterner {
        &self.interner
    }

    /// Returns a mutable reference to the string interner.
    pub fn interner_mut(&mut self) -> &mut SymbolInterner {
        &mut self.interner
    }

    /// Consumes and returns the `StringInterner`, leaving `None` in its place.
    ///
    /// # Returns
    ///
    /// The taken [`SymbolInterner`].
    pub fn take_interner(&mut self) -> SymbolInterner {
        std::mem::take(&mut self.interner)
    }

    /// Creates a new [`FinalizationContext`] by deriving it from the current semantic context.
    ///
    /// This is a lightweight operation that extracts the symbol interner and the
    /// source identifier to create a read-only environment for transformation finalization.
    ///
    /// # Arguments
    ///
    /// * `provider` - The entity responsible for the upcoming transformation
    ///   (e.g., `Provider::Analyzer` or `Provider::Optimizer`).
    ///
    /// # Examples
    ///
    /// ```
    /// let pass_ctx = semantic_ctx.as_pass_context(Provider::Analyzer);
    /// ```
    pub fn as_finalization_context(&self, provider: Provider) -> FinalizationContext {
        FinalizationContext::new(self.interner(), self.source(), provider)
    }

    pub fn as_pass_context(&self, provider: Provider) -> PassContext {
        PassContext::new(self.syntax_tree(), self.interner(), self.source(), provider)
    }

    /// Creates a new [`CheckContext`] by deriving it from the current unified context.
    ///
    /// This is a lightweight, zero-copy operation that provides a read-only environment
    /// for semantic validation, including full access to PDDL requirements and their triggers.
    ///
    /// # Arguments
    ///
    /// * `provider` - The entity performing the check (e.g., `Provider::Analyzer`).
    ///
    /// # Returns
    ///
    /// A [`CheckContext`] instance borrowing all necessary metadata from this context.
    #[inline]
    pub fn as_check_context(&self, provider: Provider) -> CheckContext {
        CheckContext::new(
            self.syntax_tree(),
            self.interner(),
            self.source(),
            provider,
            self.declared_requirements(),
        )
    }

    /// Remaps identifiers and literals in this semantic context.
    ///
    /// This method updates all identifier references within the AST and symbol table
    /// using `ident_map`, and updates the source literal using `literal_map`.
    /// It ensures consistency with a global interner.
    ///
    /// **Important:** Does not modify the string interner itself; only updates
    /// identifiers and literal references.
    ///
    /// # Arguments
    ///
    /// * `ident_map` - Maps local identifiers (`Ident`) to global identifiers.
    /// * `literal_map` - Maps local literals (`Literal`) to global literals.
    ///
    /// # Returns
    ///
    /// `Ok(())` if remapping succeeds, otherwise a [`SemanticError`] wrapping
    /// a `SymbolTableError` or `InternerError`.
    pub fn remap(
        &mut self,
        ident_map: &HashMap<SymbolId, SymbolId>,
        literal_map: &HashMap<LiteralId, LiteralId>,
    ) -> Result<(), SemanticError> {
        // Step 1: remap identifiers in AST directly
        self.syntax_tree.remap_idents(ident_map)?;

        // Step 2: remap identifiers in the symbol table directly
        self.symbol_table.remap_symbol(ident_map)?;

        // Step 3: remap the source literal
        let new_source_id = literal_map
            .get(&self.source)
            .ok_or_else(|| InternerError::missing_literal(self.source))?;
        self.source = *new_source_id;

        Ok(())
    }

    pub fn split_all_mut(&mut self) -> (&mut SymbolTable, &mut Tree<AstNode>, &mut SymbolInterner) {
        (
            &mut self.symbol_table,
            &mut self.syntax_tree,
            &mut self.interner,
        )
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
        writeln!(f, "Source: {}", self.source_name_string())?;

        let duration_since_epoch = self
            .generated_at
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::new(0, 0));

        writeln!(
            f,
            "Generated at: {} seconds since UNIX epoch\n",
            duration_since_epoch.as_secs()
        )?;

        // --- Requirements Section ---
        writeln!(f, "Declared Requirements:")?;
        if self.declared_requirements.is_empty() {
            writeln!(f, "  (none)")?;
        } else {
            for req in &self.declared_requirements {
                writeln!(f, "  - {}", req)?;
            }
        }

        writeln!(f, "\nInferred Requirements (and their triggers):")?;
        match &self.requirement_triggers {
            // Case 1: The analysis has not been performed yet (e.g., a standalone Problem)
            None => {
                writeln!(f, "  (analysis pending)")?;
            }
            // Case 2: Analysis was completed, but no specific requirements were detected
            Some(triggers) if triggers.is_empty() => {
                writeln!(f, "  (none)")?;
            }
            // Case 3: Display requirements and their associated AST NodeIds
            Some(triggers) => {
                for (req, nodes) in triggers {
                    let node_ids: Vec<String> =
                        nodes.iter().map(|id| format!("{:?}", id)).collect();
                    writeln!(
                        f,
                        "  - {}: triggered at nodes [{}]",
                        req,
                        node_ids.join(", ")
                    )?;
                }
            }
        }

        // --- Data Structures Section ---
        writeln!(f, "\nAbstract Syntax Tree Structure:")?;
        writeln!(f, "{}", self.syntax_tree)?;

        writeln!(f, "\nSymbol Table Content:")?;
        writeln!(f, "{}", self.symbol_table)?;

        Ok(())
    }
}

impl SerdeSerializable for Context {}
