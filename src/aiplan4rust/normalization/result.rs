//! Module for AST logic results and diagnostics management.
//!
//! This module defines the [`NormalizerResult`] enum, which encapsulates
//! the outcome of an AST logic phase in the `aiplan4rust` pipeline.
//!
//! # Purpose
//!
//! `NormalizerResult` bundles together:
//! - The normalized [`Ast`] if logic was successful.
//! - A [`DiagnosticManager`] that collects warnings, errors, and info messages
//!   generated during logic.
//!
//! This allows users to proceed with semantic analysis only if logic
//! succeeded, while also accessing any diagnostics that arose.

use std::fmt;

use crate::aiplan4rust::support::diagnostic::DiagnosticManager;
use crate::aiplan4rust::support::interner::SymbolInterner;
use crate::aiplan4rust::syntax::ast::Ast;

/// Represents the result of the AST logic phase.
///
/// This enum has two variants:
/// - `Success` — logic succeeded and produced a normalized AST.
/// - `Failure` — logic failed; diagnostics and the interner are preserved.
#[derive(Debug, Clone)]
pub enum Result {
    /// Normalization succeeded.
    Success {
        /// The normalized AST.
        ast: Ast,
        /// Diagnostics collected during logic.
        diagnostic_manager: DiagnosticManager,
    },
    /// Normalization failed.
    Failure {
        /// Diagnostics collected during logic.
        diagnostic_manager: DiagnosticManager,
        /// The string interner used during logic.
        interner: SymbolInterner,
    },
}

impl Result {
    /// Creates a successful logic result.
    ///
    /// # Parameters
    /// - `ast` — The normalized AST.
    /// - `diagnostic_manager` — Diagnostics collected during logic.
    ///
    /// # Returns
    /// A `NormalizerResult::Success` variant.
    pub fn success(ast: Ast, diagnostic_manager: DiagnosticManager) -> Self {
        Self::Success {
            ast,
            diagnostic_manager,
        }
    }

    /// Creates a failed logic result.
    ///
    /// # Parameters
    /// - `diagnostic_manager` — Diagnostics explaining the failure.
    /// - `interner` — The interner used during logic.
    ///
    /// # Returns
    /// A `NormalizerResult::Failure` variant.
    pub fn failure(diagnostic_manager: DiagnosticManager, interner: SymbolInterner) -> Self {
        Self::Failure {
            diagnostic_manager,
            interner,
        }
    }

    /// Returns a reference to the normalized AST if available.
    pub fn ast(&self) -> Option<&Ast> {
        match self {
            Self::Success { ast, .. } => Some(ast),
            Self::Failure { .. } => None,
        }
    }

    /// Returns a mutable reference to the normalized AST if available.
    pub fn ast_mut(&mut self) -> Option<&mut Ast> {
        match self {
            Self::Success { ast, .. } => Some(ast),
            Self::Failure { .. } => None,
        }
    }

    /// Takes ownership of the normalized AST, leaving `None` in the `Success` variant.
    pub fn take_ast(&mut self) -> Option<Ast> {
        match self {
            Self::Success { ast, .. } => Some(std::mem::take(ast)),
            Self::Failure { .. } => None,
        }
    }

    /// Returns a reference to the diagnostic manager.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        match self {
            Self::Success {
                diagnostic_manager, ..
            } => diagnostic_manager,
            Self::Failure {
                diagnostic_manager, ..
            } => diagnostic_manager,
        }
    }

    /// Returns a mutable reference to the diagnostic manager.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        match self {
            Self::Success {
                diagnostic_manager, ..
            } => diagnostic_manager,
            Self::Failure {
                diagnostic_manager, ..
            } => diagnostic_manager,
        }
    }

    /// Consumes and returns the diagnostic manager.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        match self {
            Self::Success {
                diagnostic_manager, ..
            } => std::mem::take(diagnostic_manager),
            Self::Failure {
                diagnostic_manager, ..
            } => std::mem::take(diagnostic_manager),
        }
    }

    /// Returns a reference to the interner.
    ///
    /// For `Success`, the interner is retrieved from the AST.
    /// For `Failure`, it is returned directly.
    pub fn interner(&self) -> &SymbolInterner {
        match self {
            Self::Success { ast, .. } => ast.interner(),
            Self::Failure { interner, .. } => interner,
        }
    }

    /// Returns a mutable reference to the interner.
    pub fn interner_mut(&mut self) -> &mut SymbolInterner {
        match self {
            Self::Success { ast, .. } => ast.interner_mut(),
            Self::Failure { interner, .. } => interner,
        }
    }

    /// Consumes and returns the interner.
    pub fn take_interner(&mut self) -> SymbolInterner {
        match self {
            Self::Success { ast, .. } => std::mem::take(ast.interner_mut()),
            Self::Failure { interner, .. } => std::mem::take(interner),
        }
    }

    /// Returns `true` if logic succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Returns `true` if logic failed.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure { .. })
    }
}

impl fmt::Display for Result {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success {
                ast,
                diagnostic_manager,
            } => {
                writeln!(f, "Normalization successful:\n{}", ast.syntax_tree())?;
                if !diagnostic_manager.is_empty() {
                    writeln!(f, "\nDiagnostics:")?;
                    for diag in diagnostic_manager.diagnostics() {
                        writeln!(f, "{}", diag)?;
                    }
                } else {
                    writeln!(f, "\nNo diagnostics reported.")?;
                }
            }
            Self::Failure {
                diagnostic_manager, ..
            } => {
                writeln!(f, "Normalization failed.")?;
                for diag in diagnostic_manager.diagnostics() {
                    writeln!(f, "{}", diag)?;
                }
            }
        }
        Ok(())
    }
}
