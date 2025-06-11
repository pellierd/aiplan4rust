use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::syntax::ast::Ast;
use std::fmt;

/// Represents the result of the AST normalization phase.
///
/// It contains:
/// - The normalized [`Ast`] (always present if normalization succeeds),
/// - The [`DiagnosticManager`] collecting warnings or issues encountered during normalization.
///
/// This is used after parsing but before semantic analysis.
#[derive(Debug, Clone)]
pub struct NormalizerResult {
    ast: Option<Ast>,
    diagnostic_manager: DiagnosticManager,
}

impl NormalizerResult {
    /// Constructs a new `NormalizerResult`.
    pub fn new(ast: Option<Ast>, diagnostic_manager: DiagnosticManager) -> Self {
        Self { ast, diagnostic_manager }
    }

    /// Returns a reference to the normalized AST.
    pub fn ast(&self) -> &Option<Ast> {
        &self.ast
    }

    /// Returns a mutable reference to the normalized AST.
    pub fn ast_mut(&mut self) -> &mut Option<Ast> {
        &mut self.ast
    }

    pub fn take_ast(&mut self) -> Option<Ast> {
        self.ast.take()
    }

    // Prend la possession des diagnostics
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        std::mem::take(&mut self.diagnostic_manager)
    }

    /// Returns a reference to the diagnostic manager.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the diagnostic manager.
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
