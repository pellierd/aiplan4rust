//! Provides the [`Normalizer`] struct, which performs expr of an [`Ast`] (Abstract Syntax Tree)
//! as part of the AI syntax pipeline.
//!
//! Normalization is a crucial preprocessing stage that transforms the parsed AST into a cleaner,
//! canonical form suitable for later compilation, optimization, or reasoning. It ensures consistency
//! and eliminates syntactic variations that may hinder interpretation.
//!
//! This module supports:
//! - A fixed sequence of expr normalization (e.g., typed list flattening, disjunction simplification).
//! - Internal and external [`DiagnosticManager`] support for error/warning reporting.
//! - Graceful failure with detailed diagnostics on malformed or unsupported constructs.
//!
//! # Normalization Passes
//! The expr process applies the following transformations, in order:
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
//! use aiplan4rust::expr::Normalizer;
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
//! - Internal logic bugs or malformed AST states
//!
//! Even in failure, collected diagnostics can provide useful context for recovery or debugging.

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::normalization::error::NormalizationError;
use crate::aiplan4rust::normalization::passes;
use crate::aiplan4rust::normalization::NormalizerResult;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::syntax::ParserResult;
use crate::aiplan4rust::validation::normalization::check_well_normalized;

/// Performs AST expr by applying canonical transformation normalization.
///
/// The `Normalizer` collects diagnostics and reports expr errors through
/// [`NormalizationError`] or the internal [`DiagnosticManager`]. It can be reused across multiple
/// expr operations.
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
    /// Normalizes the result of parsing by applying standard expr normalization.
    ///
    /// This method consumes the input [`ParserResult`], extracts the AST if present,
    /// applies expr normalization on it, and returns a [`NormalizerResult`] with the
    /// normalized AST and any diagnostics. If the parsing result contains no AST,
    /// expr is skipped and an error is returned.
    ///
    /// # Arguments
    ///
    /// * `parser_result` - The [`ParserResult`] to normalize.
    ///
    /// # Returns
    ///
    /// * `Ok(NormalizerResult)` if expr succeeds.
    /// * `Err(NormalizationError)` if the parsing result contains no AST or
    ///   if any expr pass fails irrecoverably.
    pub fn normalize(&mut self, mut parser_result: ParserResult) -> Result<NormalizerResult, NormalizationError> {
        match parser_result.take_ast() {
            Some(raw_ast) => {
                // Add diagnostics collected during parsing to the current diagnostic manager
                self.diagnostic_manager.add_diagnostic_from(parser_result.take_diagnostic_manager());

                // Perform expr on the extracted raw AST
                let normalizer_result = self.perform_normalization(raw_ast)?;

                // If expr produced a normalized AST, verify it is well-formed
                if let Some(normalized_ast) = normalizer_result.ast() {
                    check_well_normalized(normalized_ast)?;
                }

                // Return the successful expr result
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

    /// Internal method: orchestrates the expr pipeline.
    ///
    /// Applies a fixed sequence of normalization that simplify the AST in place.
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
        passes::normalize_typed_list(&mut ast)?;
        passes::normalize_either_type(&mut ast, &mut self.diagnostic_manager)?;
        passes::normalize_require_def(&mut ast, &mut self.diagnostic_manager)?;
        passes::normalize_type_def(&mut ast, &mut self.diagnostic_manager)?;
        Ok(NormalizerResult::success(ast, std::mem::take(&mut self.diagnostic_manager)))
    }

    /// Returns a reference to the internal [`DiagnosticManager`] for inspection or reuse.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }
}
