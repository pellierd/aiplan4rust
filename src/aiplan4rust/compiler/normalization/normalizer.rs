//! Provides the [`Normalizer`] struct, which performs logic of an [`Ast`] (Abstract Syntax Tree)
//! as part of the AI syntax pipeline.
//!
//! Normalization is a crucial preprocessing stage that transforms the parsed AST into a cleaner,
//! canonical form suitable for later compilation, optimization, or reasoning. It ensures consistency
//! and eliminates syntactic variations that may hinder interpretation.
//!
//! This module supports:
//! - A fixed sequence of logic logic (e.g., typed list flattening, disjunction simplification).
//! - Internal and external [`DiagnosticManager`] support for error/warning reporting.
//! - Graceful failure with detailed diagnostics on malformed or unsupported constructs.
//!
//! # Normalization Passes
//! The logic process applies the following transformations, in order:
//! 1. `normalize_typed_list`: Converts complex typed list syntax into a uniform structure.
//! 2. `normalize_either_type`: Resolves `either` types into disjunctions or intersections.
//! 3. `normalize_require_def`: Processes `:requirements` to ensure semantic validity.
//! 4. `normalize_type_def`: Normalizes type_checker hierarchies and definitions.
//!
//! Each pass may emit diagnostics and may return a [`NormalizationError`] if it encounters
//! an unrecoverable issue.
//!
//! # Example
//! ```rust
//! use aiplan4rust::logic::Normalizer;
//! use aiplan4rust::syntax::ast::Ast;
//!
//! let ast: Ast = /* parsed from input */;
//! let mut normalizer = Normalizer::new();
//!
//! match normalizer.normalize(ast) {
//!     Ok(result) => println!("Normalized successfully"),
//!     Err(e) => eprintln!("Normalization failed: {e}"),
//! }
//! ```
//!
//! # Error Handling
//! Errors are represented via [`NormalizationError`], which may wrap:
//! - Syntax tree violations ([`SyntaxTreeError`])
//! - Arena allocation failures ([`ArenaError`])
//! - Interning resolution errors ([`InternerError`])
//! - Internal ops bugs or malformed AST states
//!
//! Even in failure, collected diagnostics can provide useful context for recovery or debugging.

use crate::aiplan4rust::compiler::normalization::error::NormalizationError;
use crate::aiplan4rust::compiler::normalization::passes;
#[cfg(debug_assertions)]
use crate::aiplan4rust::compiler::normalization::validation::check_well_normalized;
use crate::aiplan4rust::compiler::normalization::NormalizerResult;
use crate::aiplan4rust::compiler::syntax::ast::{Ast, AstKind};
use crate::aiplan4rust::compiler::syntax::ParserResult;
use crate::aiplan4rust::support::diagnostic::DiagnosticManager;

/// Performs AST logic by applying canonical transformation logic.
///
/// The `Normalizer` collects diagnostics and reports logic errors through
/// [`NormalizationError`] or the internal [`DiagnosticManager`]. It can be reused across multiple
/// logic operations.
#[derive(Debug, Clone, Default)]
pub struct Normalizer {
    diagnostic_manager: DiagnosticManager,
}

impl Normalizer {
    /// Constructs a new `Normalizer` with an empty internal diagnostic manager.
    ///
    /// # Returns
    /// A new instance of `Normalizer`.
    pub fn new() -> Self {
        Self {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }
    /// Normalizes the result of parsing by applying standard logic logic.
    ///
    /// This method consumes the input [`ParserResult`], extracts the AST if present,
    /// applies logic logic on it, and returns a [`NormalizerResult`] with the
    /// normalized AST and any diagnostics. If the parsing result contains no AST,
    /// logic is skipped and an error is returned.
    ///
    /// # Arguments
    ///
    /// * `parser_result` - The [`ParserResult`] to normalize.
    ///
    /// # Returns
    ///
    /// * `Ok(NormalizerResult)` if logic succeeds.
    /// * `Err(NormalizationError)` if the parsing result contains no AST or
    ///   if any logic pass fails irrecoverably.
    pub fn normalize(
        &mut self,
        mut parser_result: ParserResult,
    ) -> Result<NormalizerResult, NormalizationError> {
        match parser_result.take_ast() {
            Some(raw_ast) => {
                // Add diagnostics collected during parsing to the current diagnostic manager
                self.diagnostic_manager
                    .add_diagnostic_from(parser_result.take_diagnostic_manager());

                // Perform logic on the extracted raw AST
                let normalizer_result = self.perform_normalization(raw_ast)?;

                // --- Validation Step (Debug Only) ---
                // We verify that the normalization process produced a logically sound AST.
                // This catches internal bugs in normalization finalization before they reach
                // the code generation or grounding stages.
                #[cfg(debug_assertions)]
                if let Some(normalized_ast) = normalizer_result.ast() {
                    check_well_normalized(normalized_ast)?;
                }

                // Return the successful logic result
                Ok(normalizer_result)
            }
            None => {
                // If no AST was produced during parsing, create a failure NormalizerResult
                let diagnostic_manager = parser_result.take_diagnostic_manager();
                let interner = parser_result.take_interner();
                Ok(NormalizerResult::failure(diagnostic_manager, interner))
            }
        }
    }

    /// Internal method: orchestrates the logic pipeline.
    ///
    /// Applies a fixed sequence of logic that simplification the AST in place.
    /// Any diagnostics encountered during the process are accumulated internally.
    ///
    /// # Arguments
    /// * `ast` - The mutable AST to normalize.
    ///
    /// # Returns
    /// `Ok(NormalizerResult)` on success, otherwise a [`NormalizationError`].
    fn perform_normalization(
        &mut self,
        mut ast: Ast,
    ) -> Result<NormalizerResult, NormalizationError> {
        // --- 1. Common finalization (Independent of file type) ---
        // These work on the general structure (e.g., converting (either a b)
        // or ensuring :typing requirements are consistent).
        passes::normalize_typed_list(&mut ast)?;
        passes::normalize_either_type(&mut ast, &mut self.diagnostic_manager)?;
        passes::normalize_require_def(&mut ast, &mut self.diagnostic_manager)?;

        // --- 2. Content-specific finalization ---
        // We extract the root node. If it's missing, it's a structural failure.
        let root = ast
            .syntax_tree()
            .root_node()
            .ok_or_else(NormalizationError::missing_root)?;

        // We dispatch normalization finalization based on the root kind (Domain vs Problem).
        match root.kind() {
            AstKind::Domain => {
                passes::normalize_def(&mut ast, &mut self.diagnostic_manager, AstKind::TypesDef)?;
                passes::normalize_def(
                    &mut ast,
                    &mut self.diagnostic_manager,
                    AstKind::ConstantsDef,
                )?;
                passes::normalize_def(
                    &mut ast,
                    &mut self.diagnostic_manager,
                    AstKind::FunctionsDef,
                )?;
            }
            AstKind::Problem => {
                passes::normalize_def(&mut ast, &mut self.diagnostic_manager, AstKind::ObjectsDef)?;
            }
            // If the root is neither a Domain nor a Problem, it's an incompatible structure.
            found => return Err(NormalizationError::incompatible_root(found)),
        }

        Ok(NormalizerResult::success(
            ast,
            std::mem::take(&mut self.diagnostic_manager),
        ))
    }

    /// Returns a reference to the internal [`DiagnosticManager`] for inspection or reuse.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }
}
