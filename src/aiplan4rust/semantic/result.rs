use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::support::diagnostic::DiagnosticManager;
use crate::aiplan4rust::support::interner::SymbolInterner;
use std::fmt;

/// Represents the outcome of semantic analysis, including the semantic context and diagnostics.
///
/// The `AnalyzerResult` always preserves a `StringInterner` for consistent symbol resolution,
/// whether analysis succeeded or failed.
#[derive(Debug, Clone)]
pub enum Result {
    /// Analysis succeeded, contains the semantic context and diagnostics.
    Success {
        context: SemanticContext,
        diagnostic_manager: DiagnosticManager,
    },
    /// Analysis failed, contains diagnostics and the interner.
    Failure {
        diagnostic_manager: DiagnosticManager,
        interner: SymbolInterner,
    },
}

impl Result {
    /// Creates a successful `AnalyzerResult` with a semantic context.
    pub fn success(context: SemanticContext, diagnostic_manager: DiagnosticManager) -> Self {
        Result::Success {
            context,
            diagnostic_manager,
        }
    }

    /// Creates a failure `AnalyzerResult` with diagnostics and interner.
    pub fn failure(diagnostic_manager: DiagnosticManager, interner: SymbolInterner) -> Self {
        Result::Failure {
            diagnostic_manager,
            interner,
        }
    }

    /// Returns a reference to the semantic context if analysis succeeded.
    pub fn semantic_context(&self) -> Option<&SemanticContext> {
        match self {
            Result::Success { context, .. } => Some(context),
            Result::Failure { .. } => None,
        }
    }

    /// Returns a mutable reference to the semantic context if analysis succeeded.
    pub fn semantic_context_mut(&mut self) -> Option<&mut SemanticContext> {
        match self {
            Result::Success { context, .. } => Some(context),
            Result::Failure { .. } => None,
        }
    }

    /// Takes ownership of the semantic context if analysis succeeded.
    pub fn take_semantic_context(&mut self) -> Option<SemanticContext> {
        match self {
            Result::Success { context, .. } => Some(std::mem::take(context)),
            Result::Failure { .. } => None,
        }
    }

    /// Returns a reference to the diagnostic manager.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        match self {
            Result::Success {
                diagnostic_manager, ..
            } => diagnostic_manager,
            Result::Failure {
                diagnostic_manager, ..
            } => diagnostic_manager,
        }
    }

    /// Returns a mutable reference to the diagnostic manager.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        match self {
            Result::Success {
                diagnostic_manager, ..
            } => diagnostic_manager,
            Result::Failure {
                diagnostic_manager, ..
            } => diagnostic_manager,
        }
    }

    /// Takes ownership of the diagnostic manager.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        match self {
            Result::Success {
                diagnostic_manager, ..
            } => std::mem::take(diagnostic_manager),
            Result::Failure {
                diagnostic_manager, ..
            } => std::mem::take(diagnostic_manager),
        }
    }

    /// Returns a reference to the interner used during analysis.
    ///
    /// Always available, even in case of failure.
    pub fn interner(&self) -> &SymbolInterner {
        match self {
            Result::Success { context, .. } => context.interner(),
            Result::Failure { interner, .. } => interner,
        }
    }

    /// Returns a mutable reference to the interner used during analysis.
    ///
    /// Always available, even in case of failure.
    pub fn interner_mut(&mut self) -> &mut SymbolInterner {
        match self {
            Result::Success { context, .. } => context.interner_mut(),
            Result::Failure { interner, .. } => interner,
        }
    }

    /// Consumes and returns the interner used during analysis.
    ///
    /// Always available, even in case of failure.
    pub fn take_interner(&mut self) -> SymbolInterner {
        match self {
            Result::Success { context, .. } => std::mem::take(context.interner_mut()),
            Result::Failure { interner, .. } => std::mem::take(interner),
        }
    }

    /// Returns `true` if analysis succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, Result::Success { .. })
    }

    /// Returns `true` if analysis failed.
    pub fn is_failure(&self) -> bool {
        matches!(self, Result::Failure { .. })
    }
}

impl fmt::Display for Result {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Result::Success {
                context,
                diagnostic_manager,
            } => {
                writeln!(f, "Semantic analysis succeeded:\n{}", context)?;
                if !diagnostic_manager.is_empty() {
                    writeln!(f, "\nDiagnostics:")?;
                    for diag in diagnostic_manager.diagnostics() {
                        writeln!(f, "{}", diag)?;
                    }
                } else {
                    writeln!(f, "\nNo diagnostics reported.")?;
                }
            }
            Result::Failure {
                diagnostic_manager, ..
            } => {
                writeln!(f, "Semantic analysis failed.")?;
                for diag in diagnostic_manager.diagnostics() {
                    writeln!(f, "{}", diag)?;
                }
            }
        }
        Ok(())
    }
}
