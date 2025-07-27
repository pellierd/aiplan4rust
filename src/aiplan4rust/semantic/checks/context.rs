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

use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::semantic::{SemanticContext, SymbolTable};
use std::collections::HashSet;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::tree::SyntaxTree;

/// A lightweight wrapper to pass semantic context components to verification functions.
///
/// `Context` allows semantic check functions to be reused across multiple phases,
/// including standalone semantic analysis and cross-file linking. It encapsulates the minimal
/// data required for semantic verification — without needing the full `SemanticContext`.
///
/// # Why This Wrapper?
/// In standard semantic analysis, all relevant data comes from a single file and is contained
/// in a `SemanticContext`. However, during **linking**, components (AST, symbol table, interner)
/// may come from **different files** (e.g., domain and problem).
///
/// `Context` enables you to:
/// - **Reuse** existing semantic verification logic
/// - **Inject** arbitrary components like renamed ASTs or merged symbol tables
/// - Avoid reconstructing or mutating `SemanticContext` instances just to run checks
///
/// # When to Use
/// - During verification of a single parsed file (via `from_semantic_context`)
/// - During linking, when validating problem elements using domain definitions
/// - When working with transformed ASTs or custom symbol tables
///
/// # Benefits
/// - Encourages modular, testable verification functions
/// - Reduces boilerplate in check function signatures
/// - Clarifies the dependency contract of a verification pass
///
/// # Example
/// ```rust
/// fn check_unused_symbols(ctx: &Context) {
///     let symbols = ctx.symbol_table();
///     // Perform check logic...
/// }
/// ```
///
/// # Fields
/// - `syntax_tree`: The abstract syntax arena for the context.
/// - `symbols`: The symbol table used during resolution.
/// - `interner`: The global string interner for identifiers.
/// - `source_name`: The name of the source file (used for diagnostics).
/// - `requirements`: Active requirements (e.g., :typing, :durative-actions).
#[derive(Clone)]
pub struct Context<'a> {
    syntax_tree: &'a SyntaxTree<AstNode>,
    symbols: &'a SymbolTable,
    interner: &'a StringInterner,
    source_name: &'a str,
    requirements: &'a HashSet<Requirement>,
}

impl<'a> Context<'a> {
    /// Creates a new `Context` from individual components.
    pub fn new(
        syntax_tree: &'a SyntaxTree<AstNode>,
        symbols: &'a SymbolTable,
        interner: &'a StringInterner,
        source_name: &'a str,
        requirements: &'a HashSet<Requirement>,
    ) -> Self {
        Self {
            syntax_tree,
            symbols,
            interner,
            source_name,
            requirements,
        }
    }

    /// Returns the AST.
    pub fn syntax_tree(&self) -> &'a SyntaxTree<AstNode> {
        self.syntax_tree
    }

    /// Returns the symbol table.
    pub fn symbol_table(&self) -> &'a SymbolTable {
        self.symbols
    }

    /// Returns the string interner.
    pub fn interner(&self) -> &'a StringInterner {
        self.interner
    }

    /// Returns the name of the source file.
    pub fn source_name(&self) -> &'a str {
        self.source_name
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
            source_name: &ctx.source_name(),
            requirements: &ctx.requirements(),
        }
    }
}
