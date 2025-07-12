use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::syntax::ast::AstArena;
use std::fmt;

/// Represents the result of the AST normalization phase.
///
/// This struct encapsulates both the normalized [`Ast`] (if normalization succeeded)
/// and the [`DiagnosticManager`] which collects all warnings, errors,
/// or informational messages generated during normalization.
///
/// The `NormalizerResult` is typically produced after parsing and normalization
/// but before semantic analysis.
///
/// # Structure
/// - `ast`: An optional normalized AST. This is `Some(ast)` if normalization
///   was successful, otherwise `None`.
/// - `diagnostic_manager`: Holds diagnostics produced during normalization.
///
/// # Usage
///
/// After normalization, users can inspect the normalized AST and any
/// diagnostics to determine if further processing should proceed.
///
/// ```rust
/// let result: NormalizerResult = normalizer.normalize(ast)?;
///
/// if let Some(normalized_ast) = result.ast() {
///     // Use normalized AST
/// }
///
/// for diag in result.diagnostic_manager().diagnostics() {
///     println!("Diagnostic: {}", diag);
/// }
/// ```
#[derive(Debug)]
pub struct NormalizerResult {
    ast: Option<AstArena>,
    diagnostic_manager: DiagnosticManager,
}

impl NormalizerResult {
    /// Constructs a new `NormalizerResult`.
    ///
    /// # Arguments
    ///
    /// * `ast` - An optional normalized AST. `Some(ast)` if normalization succeeded,
    ///   otherwise `None`.
    /// * `diagnostic_manager` - The diagnostic manager capturing any diagnostics.
    ///
    /// # Returns
    ///
    /// A new instance of `NormalizerResult`.
    pub fn new(ast: Option<AstArena>, diagnostic_manager: DiagnosticManager) -> Self {
        Self { ast, diagnostic_manager }
    }

    /// Returns an immutable reference to the normalized AST.
    ///
    /// # Returns
    ///
    /// A reference to the optional normalized AST. If normalization failed,
    /// this will be `None`.
    pub fn ast(&self) -> &Option<AstArena> {
        &self.ast
    }

    /// Returns a mutable reference to the normalized AST.
    ///
    /// This allows modification or replacement of the AST within the result.
    ///
    /// # Returns
    ///
    /// A mutable reference to the optional normalized AST.
    pub fn ast_mut(&mut self) -> &mut Option<AstArena> {
        &mut self.ast
    }

    /// Takes ownership of the normalized AST, leaving `None` in its place.
    ///
    /// This is useful when transferring ownership out of the result.
    ///
    /// # Returns
    ///
    /// The normalized AST if present, or `None`.
    pub fn take_ast(&mut self) -> Option<AstArena> {
        self.ast.take()
    }

    /// Takes ownership of the diagnostic manager, replacing it with an empty one.
    ///
    /// # Returns
    ///
    /// The `DiagnosticManager` instance containing collected diagnostics.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        std::mem::take(&mut self.diagnostic_manager)
    }

    /// Returns an immutable reference to the diagnostic manager.
    ///
    /// Allows inspection of warnings, errors, or informational diagnostics
    /// collected during normalization.
    ///
    /// # Returns
    ///
    /// Reference to the internal `DiagnosticManager`.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the diagnostic manager.
    ///
    /// Allows adding or modifying diagnostics after normalization.
    ///
    /// # Returns
    ///
    /// Mutable reference to the internal `DiagnosticManager`.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }
}

impl fmt::Display for NormalizerResult {
    /// Formats the normalization result for display.
    ///
    /// If an AST is present, it prints the root node of the AST.
    /// It then prints any diagnostics collected during normalization.
    ///
    /// If no AST is present, it notes that normalization failed.
    ///
    /// # Example output
    ///
    /// ```
    /// Normalized AST:
    /// (AST root node printed here)
    ///
    /// Normalization diagnostics:
    /// - Warning: ...
    /// - Error: ...
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.ast {
            Some(ast) => {
                writeln!(f, "Normalized AST:\n{}", ast.arena())?;
            }
            None => {
                writeln!(f, "No AST available (normalization failed).")?;
            }
        }

        if self.diagnostic_manager.is_empty() {
            writeln!(f, "\nNo issues during normalization.")
        } else {
            writeln!(f, "\nNormalization diagnostics:")?;
            for diag in self.diagnostic_manager.diagnostics() {
                writeln!(f, "- {}", diag)?;
            }
            Ok(())
        }
    }
}
