//! Module handling the lightweight semantic `Context` wrapper.
//!
//! This module provides the `Context` struct, which serves as a minimal view into the semantic
//! analysis components such as the AST, symbol table, interner, and requirements.
//!
//! It enables reuse of semantic verification logic in various scenarios, including
//! single-file semantic checks and multi-file linking phases where components
//! might come from different sources.
//!
//! The module also includes convenient conversions from the full `SemanticContext`,
//! allowing flexible and modular semantic analysis workflows.

use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{LiteralId, Requirement};
use crate::aiplan4rust::semantic::{SemanticContext, SymbolTable};
use std::collections::HashSet;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::Tree;

/// A lightweight wrapper that provides semantic context components to verification functions.
///
/// `Context` enables semantic checks to be reused across different phases —
/// such as standalone semantic analysis and cross-file linking — by bundling only
/// the essential components needed for verification, without requiring the full `SemanticContext`.
///
/// # Purpose
///
/// In traditional semantic analysis, all required data (syntax tree, symbol table, etc.)
/// comes from a single file and is encapsulated in a `SemanticContext`. However, during **linking**,
/// these components may originate from different sources — for example, when verifying a problem file
/// against a domain file.
///
/// `Context` provides a modular solution that allows:
/// - Reuse of semantic checking logic across different phases
/// - Injection of custom or merged components (e.g., merged symbol tables)
/// - Avoidance of unnecessary reconstruction or mutation of `SemanticContext`
///
/// # Use Cases
/// - Semantic verification of a single parsed file via `from_semantic_context()`
/// - Cross-file linking (e.g., validating a problem against its domain)
/// - Operating on transformed or synthesized ASTs
///
/// # Advantages
/// - Simplifies function signatures for verification normalization
/// - Makes verification logic more modular and testable
/// - Clearly expresses a verification function’s data dependencies
///
/// # Example
/// ```rust
/// fn check_unused_symbols(ctx: &Context) {
///     let symbols = ctx.symbol_table();
///     // perform analysis ...
/// }
/// ```
///
/// # Fields
/// - `syntax_tree`: Reference to the AST for this context.
/// - `symbols`: The symbol table used for resolution.
/// - `interner`: Global string interner for identifier lookup.
/// - `source_id`: Interned identifier of the source file (used for diagnostics).
/// - `requirements`: Set of active requirements (e.g., `:typing`, `:durative-actions`).
#[derive(Clone)]
pub struct Context<'a> {
    syntax_tree: &'a Tree<AstNode>,
    symbols: &'a SymbolTable,
    interner: &'a SymbolInterner,
    source_id: LiteralId,
    requirements: &'a HashSet<Requirement>,
}

impl<'a> Context<'a> {
    /// Creates a new [`Context`] from the given semantic components.
    ///
    /// This constructor is used when you want to manually assemble a semantic context,
    /// typically outside the standard `SemanticContext` flow — for example, during
    /// cross-file linking or when testing semantic normalization in isolation.
    ///
    /// # Parameters
    ///
    /// - `syntax_tree`: Reference to the syntax tree (`SyntaxTree<AstNode>`) containing the parsed structure.
    /// - `symbols`: Reference to the [`SymbolTable`] used for name resolution and symbol lookup.
    /// - `interner`: Reference to the [`StringInterner`] that holds all interned identifiers.
    /// - `source_id`: The interned [`Literal`] representing the source file associated with this context (used in diagnostics).
    /// - `requirements`: Reference to the set of active [`Requirement`]s enabled in this context (e.g., `:typing`, `:durative-actions`).
    ///
    /// # Returns
    ///
    /// A [`Context`] instance encapsulating all the provided components for use in semantic verification.
    ///
    /// # Example
    ///
    /// ```rust
    /// let context = Context::new(
    ///     &syntax_tree,
    ///     &symbol_table,
    ///     &interner,
    ///     source_id,
    ///     &active_requirements,
    /// );
    /// ```
    ///
    /// Use this when composing custom or merged contexts, such as when verifying a problem file
    /// with domain-level types and operators.
    ///
    /// [`Context`]: crate::Context
    /// [`SymbolTable`]: crate::semantics::SymbolTable
    /// [`StringInterner`]: crate::interner::StringInterner
    /// [`Requirement`]: crate::features::Requirement
    /// [`Literal`]: crate::interner::Literal
    pub fn new(
        syntax_tree: &'a Tree<AstNode>,
        symbols: &'a SymbolTable,
        interner: &'a SymbolInterner,
        source_id: LiteralId,
        requirements: &'a HashSet<Requirement>,
    ) -> Self {
        Self {
            syntax_tree,
            symbols,
            interner,
            source_id,
            requirements,
        }
    }

    /// Returns the AST.
    pub fn syntax_tree(&self) -> &'a Tree<AstNode> {
        self.syntax_tree
    }

    /// Returns the symbol table.
    pub fn symbol_table(&self) -> &'a SymbolTable {
        self.symbols
    }

    /// Returns the string interner.
    pub fn interner(&self) -> &'a SymbolInterner {
        self.interner
    }

    /// Returns the interned [`LiteralId`] representing the source file associated with this context.
    pub fn source_id(&self) -> LiteralId {
        self.source_id
    }

    /// Returns the active requirements.
    pub fn requirements(&self) -> &'a HashSet<Requirement> {
        self.requirements
    }
}

impl<'a> From<&'a SemanticContext> for Context<'a> {
    /// Converts a reference to a `SemanticContext` into a `Context`.
    ///
    /// This implementation allows creating a `Context` from an existing
    /// `SemanticContext` by borrowing its internal components.
    ///
    /// # Arguments
    ///
    /// * `ctx` - A reference to the `SemanticContext` to convert from.
    ///
    /// # Returns
    ///
    /// A new `Context` instance borrowing data from the provided `SemanticContext`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let semantic_ctx: SemanticContext = ...;
    /// let ctx: Context = Context::from(&semantic_ctx);
    /// ```
    fn from(ctx: &'a SemanticContext) -> Self {
        Self {
            syntax_tree: &ctx.syntax_tree(),
            symbols: &ctx.symbol_table(),
            interner: &ctx.interner(),
            source_id: ctx.source_id(),
            requirements: &ctx.declared_requirements(),
        }
    }
}
