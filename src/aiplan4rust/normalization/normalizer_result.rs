use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::syntax::ast::Ast;
use std::fmt;

/// Represents the result of the AST normalization phase.
///
/// Contains:
/// - The normalized [`Ast`] (optional, present if normalization succeeds),
/// - The [`DiagnosticManager`] that collects warnings or issues encountered during normalization.
///
/// This struct is used after parsing but before semantic analysis.
#[derive(Debug)]
pub struct NormalizerResult {
    ast: Option<Ast>,
    diagnostic_manager: DiagnosticManager,
}

impl NormalizerResult {
    /// Creates a new `NormalizerResult`.
    ///
    /// # Arguments
    /// - `ast`: An `Option` containing the normalized AST. `Some(ast)` if normalization was successful, `None` otherwise.
    /// - `diagnostic_manager`: The diagnostic manager collecting normalization diagnostics.
    pub fn new(ast: Option<Ast>, diagnostic_manager: DiagnosticManager) -> Self {
        Self { ast, diagnostic_manager }
    }

    /// Returns a reference to the normalized AST.
    ///
    /// # Returns
    /// - `&Option<Ast>`: A reference to the normalized AST, or `None` if none is available.
    pub fn ast(&self) -> &Option<Ast> {
        &self.ast
    }

    /// Returns a mutable reference to the normalized AST.
    ///
    /// # Returns
    /// - `&mut Option<Ast>`: A mutable reference allowing modification or replacement of the AST.
    pub fn ast_mut(&mut self) -> &mut Option<Ast> {
        &mut self.ast
    }

    /// Takes ownership of the normalized AST, leaving `None` in its place.
    ///
    /// # Returns
    /// - `Option<Ast>`: The normalized AST if present, or `None`.
    pub fn take_ast(&mut self) -> Option<Ast> {
        self.ast.take()
    }

    /// Takes ownership of the diagnostic manager, replacing it with an empty one.
    ///
    /// # Returns
    /// - `DiagnosticManager`: The previously held diagnostic manager.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        std::mem::take(&mut self.diagnostic_manager)
    }

    /// Returns a reference to the diagnostic manager.
    ///
    /// # Returns
    /// - `&DiagnosticManager`: An immutable reference to the diagnostic manager.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the diagnostic manager.
    ///
    /// # Returns
    /// - `&mut DiagnosticManager`: A mutable reference for adding or modifying diagnostics.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }
}

impl fmt::Display for NormalizerResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.ast {
            Some(ast) => {
                writeln!(f, "Normalized AST:\n{}", ast.root())?;
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
