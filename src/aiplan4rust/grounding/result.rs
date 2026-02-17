use crate::aiplan4rust::grounding::problem::Problem;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::interner::SymbolInterner;
use std::fmt;

/// Represents the outcome of grounding a lifted planning problem.
///
/// `GroundingResult` encapsulates both successful and failed grounding attempts,
/// providing access to the optionally grounded `Problem` and the associated
/// diagnostics collected during the grounding process.
///
/// # Variants
///
/// - `Success` — Grounding succeeded and contains a fully constructed `Problem`
///   along with a `DiagnosticManager`.
/// - `Failure` — Grounding failed, containing a `DiagnosticManager` and a
///   `StringInterner` preserving identifiers.
#[derive(Debug, Clone)]
pub enum Result {
    /// Grounding succeeded, containing the grounded problem and diagnostics.
    Success {
        /// The grounded problem generated from the lifted problem.
        problem: Problem,
        /// Diagnostics collected during grounding (warnings, info, etc.).
        diagnostic_manager: DiagnosticManager,
    },
    /// Grounding failed, containing diagnostics and the interner.
    Failure {
        /// Diagnostics explaining why grounding failed.
        diagnostic_manager: DiagnosticManager,
        /// Interner preserved for consistent symbol reporting.
        interner: SymbolInterner,
    },
}

impl Result {
    /// Creates a successful grounding result.
    pub fn success(problem: Problem, diagnostic_manager: DiagnosticManager) -> Self {
        Self::Success {
            problem,
            diagnostic_manager,
        }
    }

    /// Creates a failed grounding result with diagnostics and interner.
    pub fn failure(diagnostic_manager: DiagnosticManager, interner: SymbolInterner) -> Self {
        Self::Failure {
            diagnostic_manager,
            interner,
        }
    }

    /// Returns a reference to the grounded problem if successful.
    pub fn problem(&self) -> Option<&Problem> {
        match self {
            Self::Success { problem, .. } => Some(problem),
            Self::Failure { .. } => None,
        }
    }

    /// Returns a mutable reference to the grounded problem if successful.
    pub fn problem_mut(&mut self) -> Option<&mut Problem> {
        match self {
            Self::Success { problem, .. } => Some(problem),
            Self::Failure { .. } => None,
        }
    }

    /// Consumes and returns the grounded problem if successful.
    pub fn take_problem(&mut self) -> Option<Problem> {
        match self {
            Self::Success { problem, .. } => Some(std::mem::take(problem)),
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

    /// Consumes and returns the diagnostic manager, leaving an empty one.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        match self {
            Self::Success { diagnostic_manager, .. } => std::mem::take(diagnostic_manager),
            Self::Failure { diagnostic_manager, .. } => std::mem::take(diagnostic_manager),
        }
    }

    /// Returns a reference to the interner.
    ///
    /// - If grounding succeeded, retrieves the interner from the problem.
    /// - If grounding failed, retrieves the preserved interner.
    pub fn interner(&self) -> &SymbolInterner {
        match self {
            Self::Success { problem, .. } => problem.interner(),
            Self::Failure { interner, .. } => interner,
        }
    }

    /// Returns a mutable reference to the interner.
    pub fn interner_mut(&mut self) -> &mut SymbolInterner {
        match self {
            Self::Success { problem, .. } => problem.interner_mut(),
            Self::Failure { interner, .. } => interner,
        }
    }

    /// Consumes and returns the interner.
    pub fn take_interner(&mut self) -> SymbolInterner {
        match self {
            Self::Success { problem, .. } => std::mem::take(problem.interner_mut()),
            Self::Failure { interner, .. } => std::mem::take(interner),
        }
    }

    /// Returns true if grounding succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Returns true if grounding failed.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure { .. })
    }
}

impl fmt::Display for Result {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success { problem, diagnostic_manager } => {
                writeln!(f, "Grounding succeeded:\n{}", problem)?;
                if !diagnostic_manager.is_empty() {
                    writeln!(f, "\nDiagnostics:")?;
                    for diag in diagnostic_manager.diagnostics() {
                        writeln!(f, "{}", diag)?;
                    }
                } else {
                    writeln!(f, "\nNo diagnostics reported.")?;
                }
            }
            Self::Failure { diagnostic_manager, .. } => {
                writeln!(f, "Grounding failed.")?;
                for diag in diagnostic_manager.diagnostics() {
                    writeln!(f, "{}", diag)?;
                }
            }
        }
        Ok(())
    }
}
