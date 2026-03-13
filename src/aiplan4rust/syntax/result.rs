//! Module defining the `ParserResult` either_type, representing the outcome of a PDDL parsing operation.
//!
//! This module provides a unified enum-based result either_type for parsing,
//! including diagnostics and optional AST.

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::interner::SymbolInterner;
use std::fmt;

/// Represents the outcome of a PDDL syntax parsing operation.
///
/// `ParserResult` encapsulates either a successfully parsed AST with diagnostics,
/// or a failure with diagnostics and the interner used during parsing.
///
/// This enum-based design removes `Option` wrappers and avoids `expect()` calls,
/// making result handling more robust and explicit.
///
/// # Variants
///
/// - `Success` — parsing succeeded and contains the AST and diagnostics.
/// - `Failure` — parsing failed, contains diagnostics and the interner used for error reporting.
#[derive(Debug, Clone)]
pub enum Result {
    /// Parsing succeeded with a parsed AST and associated diagnostics.
    Success {
        ast: Ast,
        diagnostic_manager: DiagnosticManager,
    },
    /// Parsing failed; diagnostics and the interner are preserved.
    Failure {
        diagnostic_manager: DiagnosticManager,
        interner: SymbolInterner,
    },
}

impl Result {
    /// Creates a successful parser result.
    ///
    /// # Arguments
    /// * `ast` – The successfully parsed AST.
    /// * `diagnostic_manager` – Diagnostics collected during parsing.
    ///
    /// # Returns
    /// A `ParserResult::Success` variant.
    pub fn success(ast: Ast, diagnostic_manager: DiagnosticManager) -> Self {
        Self::Success { ast, diagnostic_manager }
    }

    /// Creates a failed parser result.
    ///
    /// # Arguments
    /// * `diagnostic_manager` – Diagnostics explaining why parsing failed.
    /// * `interner` – The interner used during parsing.
    ///
    /// # Returns
    /// A `ParserResult::Failure` variant.
    pub fn failure(diagnostic_manager: DiagnosticManager, interner: SymbolInterner) -> Self {
        Self::Failure { diagnostic_manager, interner }
    }

    /// Returns `true` if parsing succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Returns `true` if parsing failed.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure { .. })
    }

    /// Returns a reference to the parsed AST, if available.
    pub fn ast(&self) -> Option<&Ast> {
        match self {
            Self::Success { ast, .. } => Some(ast),
            Self::Failure { .. } => None,
        }
    }

    /// Returns a mutable reference to the parsed AST, if available.
    pub fn ast_mut(&mut self) -> Option<&mut Ast> {
        match self {
            Self::Success { ast, .. } => Some(ast),
            Self::Failure { .. } => None,
        }
    }

    /// Takes ownership of the AST, leaving nothing in its place.
    pub fn take_ast(&mut self) -> Option<Ast> {
        match self {
            Self::Success { ast, .. } => Some(std::mem::take(ast)),
            Self::Failure { .. } => None,
        }
    }

    /// Returns a reference to the diagnostic manager.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        match self {
            Self::Success { diagnostic_manager, .. } => diagnostic_manager,
            Self::Failure { diagnostic_manager, .. } => diagnostic_manager,
        }
    }

    /// Returns a mutable reference to the diagnostic manager.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        match self {
            Self::Success { diagnostic_manager, .. } => diagnostic_manager,
            Self::Failure { diagnostic_manager, .. } => diagnostic_manager,
        }
    }

    /// Takes ownership of the diagnostic manager.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        match self {
            Self::Success { diagnostic_manager, .. } => std::mem::take(diagnostic_manager),
            Self::Failure { diagnostic_manager, .. } => std::mem::take(diagnostic_manager),
        }
    }

    /// Returns a reference to the string interner.
    ///
    /// For `Success`, the interner can be retrieved from the AST.
    /// For `Failure`, it is returned from the variant.
    pub fn interner(&self) -> &SymbolInterner {
        match self {
            Self::Success { ast, .. } => ast.interner(),
            Self::Failure { interner, .. } => interner,
        }
    }

    /// Returns a mutable reference to the string interner.
    pub fn interner_mut(&mut self) -> &mut SymbolInterner {
        match self {
            Self::Success { ast, .. } => ast.interner_mut(),
            Self::Failure { interner, .. } => interner,
        }
    }

    /// Consumes and returns the string interner.
    pub fn take_interner(&mut self) -> SymbolInterner {
        match self {
            Self::Success { ast, .. } => std::mem::take(ast.interner_mut()),
            Self::Failure { interner, .. } => std::mem::take(interner),
        }
    }
}

impl fmt::Display for Result {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success { ast, diagnostic_manager } => {
                writeln!(f, "Parsing successful:\n{}", ast)?;
                if !diagnostic_manager.is_empty() {
                    writeln!(f, "\nDiagnostics encountered during parsing:")?;
                    for diag in diagnostic_manager.diagnostics() {
                        writeln!(f, "{}", diag)?;
                    }
                } else {
                    writeln!(f, "\nNo diagnostics detected.")?;
                }
            }
            Self::Failure { diagnostic_manager, interner } => {
                writeln!(f, "Parsing failed:")?;
                for diag in diagnostic_manager.diagnostics() {
                    writeln!(f, "{}", diag)?;
                }
                writeln!(f, "\nInterner contents:\n{}", interner)?;
            }
        }
        Ok(())
    }
}
