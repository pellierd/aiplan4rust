use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::normalization::passes;
use crate::aiplan4rust::normalization::NormalizerResult;
use crate::aiplan4rust::syntax::ast::{Ast, AstArena};

/// The `Normalizer` struct provides functionality to transform an Abstract Syntax Tree (AST)
/// into a standardized, normalized form suitable for further processing or compilation.
///
/// Normalization typically involves applying a sequence of transformation passes that
/// restructure, simplify, or canonicalize the AST to enforce consistency and remove ambiguities.
///
/// During normalization, the `Normalizer` also accumulates diagnostics such as warnings and errors
/// via an internal [`DiagnosticManager`], enabling detailed reporting of issues found.
///
/// # Examples
///
/// ```rust
/// use crate::aiplan4rust::syntax::ast::Ast;
/// use crate::aiplan4rust::normalization::Normalizer;
///
/// let mut normalizer = Normalizer::new();
/// let ast: Ast = /* ... obtain AST ... */;
///
/// match normalizer.normalize(ast) {
///     Ok(result) => {
///         let normalized_ast = result.ast;
///         // Proceed with normalized AST
///     }
///     Err(err) => {
///         eprintln!("Normalization failed: {:?}", err);
///     }
/// }
/// ```
///
/// # Note
///
/// The `Normalizer` owns an internal [`DiagnosticManager`], but you may also supply
/// your own diagnostic manager to collect diagnostics externally.
///
/// This is useful for integrating the normalizer into larger toolchains where
/// diagnostics need to be aggregated or customized.
#[derive(Debug, Clone, Default)]
pub struct Normalizer {
    diagnostic_manager: DiagnosticManager,
}

impl Normalizer {
    /// Constructs a new `Normalizer` instance with an empty diagnostic manager.
    ///
    /// # Returns
    ///
    /// A fresh `Normalizer` ready to normalize ASTs and collect diagnostics.
    ///
    /// # Example
    ///
    /// ```rust
    /// let normalizer = Normalizer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Normalizes the provided AST, applying all standard normalization passes.
    ///
    /// This method takes ownership of the AST, applies a fixed sequence of transformations,
    /// updates the AST in place, and collects any diagnostics internally.
    ///
    /// # Arguments
    ///
    /// * `ast` - The AST to normalize.
    ///
    /// # Returns
    ///
    /// - `Ok(NormalizerResult)` containing the normalized AST and accumulated diagnostics
    ///   if normalization succeeds.
    /// - `Err(ParserInternalError)` if an internal error occurs during any normalization pass.
    ///
    /// # Errors
    ///
    /// Errors returned are typically internal logic errors detected during normalization.
    pub fn normalize(&mut self, ast: AstArena) -> Result<NormalizerResult, ParserInternalError> {
        self.perform_normalization(ast)
    }

    /// Normalizes the AST using an externally supplied diagnostic manager.
    ///
    /// This method replaces the internal diagnostic manager with the provided one,
    /// allowing diagnostics to be collected outside of the `Normalizer`.
    ///
    /// # Arguments
    ///
    /// * `ast` - The AST to normalize.
    /// * `diagnostic_manager` - An externally created diagnostic manager for collecting diagnostics.
    ///
    /// # Returns
    ///
    /// - `Ok(NormalizerResult)` if normalization succeeds.
    /// - `Err(ParserInternalError)` if an error occurs during normalization.
    pub fn normalize_with_diagnostic_manager(
        &mut self,
        ast: AstArena,
        diagnostic_manager: DiagnosticManager,
    ) -> Result<NormalizerResult, ParserInternalError> {
        self.diagnostic_manager = diagnostic_manager;
        self.perform_normalization(ast)
    }

    /// Private helper that executes the actual normalization passes on the AST.
    ///
    /// This method applies several normalization passes, mutating the AST and
    /// recording any diagnostics. The passes include:
    ///
    /// - Typed list normalization
    /// - Either type normalization
    /// - Require definition normalization
    /// - Type definition normalization
    ///
    /// # Arguments
    ///
    /// * `ast` - The AST to normalize.
    ///
    /// # Returns
    ///
    /// - `Ok(NormalizerResult)` with the normalized AST and diagnostics.
    /// - `Err(ParserInternalError)` if any pass fails.
    fn perform_normalization(
        &mut self,
        mut ast: AstArena,
    ) -> Result<NormalizerResult, ParserInternalError> {
        passes::normalize_typed_list(&mut ast)?;
        passes::normalize_either_type(&mut ast, &mut self.diagnostic_manager)?;
        passes::normalize_require_def(&mut ast, &mut self.diagnostic_manager)?;
        //ast.arena_mut().compact_from_preorder();  passes::normalize_type_def(&mut ast, &mut self.diagnostic_manager)?;

        Ok(NormalizerResult::new(Some(ast), std::mem::take(&mut self.diagnostic_manager)))
    }

    /// Returns a reference to the internal diagnostic manager.
    ///
    /// This allows inspection of diagnostics accumulated during normalization.
    ///
    /// # Returns
    ///
    /// Reference to the `DiagnosticManager` containing warnings and errors.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }
}
