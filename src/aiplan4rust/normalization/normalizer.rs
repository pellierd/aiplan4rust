use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::normalization::passes;
use crate::aiplan4rust::normalization::NormalizerResult;
use crate::aiplan4rust::syntax::ast::Ast;

/// The `Normalizer` is responsible for transforming an AST into a standardized form.
///
/// It applies a series of normalization passes to the AST, updating it and collecting any
/// diagnostics (warnings or errors) encountered during the process.
///
/// The normalizer maintains an internal [`DiagnosticManager`] to accumulate diagnostics.
#[derive(Debug, Clone, Default)]
pub struct Normalizer {
    diagnostic_manager: DiagnosticManager,
}

impl Normalizer {
    /// Creates a new `Normalizer` instance.
    ///
    /// # Returns
    /// A fresh `Normalizer` with an empty diagnostic manager.
    pub fn new() -> Self {
        Self {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Normalizes the given AST by applying standard normalization passes.
    ///
    /// # Arguments
    /// - `ast`: The [`Ast`] to be normalized. Ownership is taken and the AST may be mutated.
    ///
    /// # Returns
    /// - `Ok(NormalizerResult)`: Contains the normalized AST and collected diagnostics if successful.
    /// - `Err(ParserInternalError)`: If any internal error occurs during normalization passes.
    ///
    /// This method accumulates diagnostics internally within the normalizer.
    pub fn normalize(&mut self, ast: Ast) -> Result<NormalizerResult, ParserInternalError> {
        self.perform_normalization(ast)
    }

    /// Normalizes the given AST with an externally provided diagnostic manager.
    ///
    /// # Arguments
    /// - `ast`: The [`Ast`] to normalize. Ownership is taken and the AST may be mutated.
    /// - `diagnostic_manager`: An externally created [`DiagnosticManager`] used to collect
    ///   diagnostics during normalization.
    ///
    /// # Returns
    /// - `Ok(NormalizerResult)`: Contains the normalized AST and collected diagnostics if
    ///   successful.
    /// - `Err(ParserInternalError)`: If an error occurs during normalization passes.
    ///
    /// This method replaces the internal diagnostic manager with the provided one,
    /// allowing diagnostics to be accumulated externally.
    pub fn normalize_with_diagnostic_manager(
        &mut self,
        ast: Ast,
        diagnostic_manager: DiagnosticManager,
    ) -> Result<NormalizerResult, ParserInternalError> {
        self.diagnostic_manager = diagnostic_manager;
        self.perform_normalization(ast)
    }

    /// Internal helper method performing the actual normalization passes.
    ///
    /// # Arguments
    /// - `ast`: The AST to normalize. This method takes ownership and mutates it.
    ///
    /// # Returns
    /// - `Ok(NormalizerResult)`: Contains the normalized AST and collected diagnostics.
    /// - `Err(ParserInternalError)`: If any pass fails.
    ///
    /// This method applies several normalization passes and assigns unique IDs to AST nodes.
    fn perform_normalization(
        &mut self,
        mut ast: Ast,
    ) -> Result<NormalizerResult, ParserInternalError> {
        passes::normalize_typed_list(&mut ast)?;
        passes::normalize_either_type(&mut ast, &mut self.diagnostic_manager)?;
        passes::normalize_require_def(&mut ast, &mut self.diagnostic_manager)?;
        passes::normalize_type_def(&mut ast, &mut self.diagnostic_manager)?;
        ast.assign_unique_ids(0);
        Ok(NormalizerResult::new(Some(ast), std::mem::take(&mut self.diagnostic_manager)))
    }

    /// Returns an immutable reference to the diagnostic manager.
    ///
    /// # Returns
    /// - `&DiagnosticManager`: A reference to the diagnostic manager holding diagnostics
    ///   collected during normalization.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }
}
