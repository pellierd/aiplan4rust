use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lir::problem::NewLiftedProblem;
use std::fmt;

/// Represents the outcome of the IR (Intermediate Representation) building phase.
///
/// `LirBuilderResult` encapsulates either a successfully constructed `LiftedProblem`
/// or a failure with diagnostics. The `StringInterner` used during the build is always
/// preserved, even in case of failure, to ensure consistent access to identifiers
/// for reporting or debugging.
///
/// # Variants
///
/// - `Success` — IR was successfully built, contains the `LiftedProblem` and diagnostics.
/// - `Failure` — IR build failed, contains diagnostics and a `StringInterner`.
#[derive(Debug, Clone)]
pub enum Result {
    /// IR was successfully built.
    Success {
        /// The constructed lifted problem.
        lifted_problem: NewLiftedProblem,
        /// Diagnostics collected during the build.
        diagnostic_manager: DiagnosticManager,
    },
    /// IR build failed.
    Failure {
        /// Diagnostics explaining why the build failed.
        diagnostic_manager: DiagnosticManager,
        /// Interner preserved for consistent symbol reporting.
        interner: SymbolInterner,
    },
}

impl Result {
    /// Creates a successful builder result containing the lifted problem and diagnostics.
    ///
    /// # Parameters
    /// - `lifted_problem` — The successfully built `LiftedProblem`.
    /// - `diagnostic_manager` — Diagnostics collected during the build.
    ///
    /// # Returns
    /// A `Result::Success` variant.
    pub fn success(
        lifted_problem: NewLiftedProblem,
        diagnostic_manager: DiagnosticManager,
    ) -> Self {
        Self::Success {
            lifted_problem,
            diagnostic_manager,
        }
    }

    /// Creates a failed builder result with diagnostics and interner.
    ///
    /// # Parameters
    /// - `diagnostic_manager` — Diagnostics explaining the failure.
    /// - `interner` — Interner used during the build for reporting or debugging.
    ///
    /// # Returns
    /// A `Result::Failure` variant.
    pub fn failure(diagnostic_manager: DiagnosticManager, interner: SymbolInterner) -> Self {
        Self::Failure {
            diagnostic_manager,
            interner,
        }
    }

    /// Returns a reference to the constructed IR if available.
    ///
    /// # Returns
    /// - `Some(&LiftedProblem)` if the build succeeded.
    /// - `None` if the build failed.
    pub fn lifted_problem(&self) -> Option<&NewLiftedProblem> {
        match self {
            Self::Success { lifted_problem, .. } => Some(lifted_problem),
            Self::Failure { .. } => None,
        }
    }

    /// Returns a mutable reference to the constructed IR if available.
    pub fn lifted_problem_mut(&mut self) -> Option<&mut NewLiftedProblem> {
        match self {
            Self::Success { lifted_problem, .. } => Some(lifted_problem),
            Self::Failure { .. } => None,
        }
    }

    /// Consumes and returns the lifted problem if available.
    pub fn take_lifted_problem(&mut self) -> Option<NewLiftedProblem> {
        match self {
            Self::Success { lifted_problem, .. } => Some(std::mem::take(lifted_problem)),
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
            Result::Success {
                diagnostic_manager, ..
            } => diagnostic_manager,
            Result::Failure {
                diagnostic_manager, ..
            } => diagnostic_manager,
        }
    }

    /// Consumes and returns the diagnostic manager, leaving an empty one.
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

    /// Returns a reference to the string interner used during the build.
    ///
    /// Always available, even in failure.
    pub fn interner(&self) -> &SymbolInterner {
        match self {
            Self::Success { lifted_problem, .. } => lifted_problem.interner(),
            Self::Failure { interner, .. } => interner,
        }
    }

    /// Returns a mutable reference to the string interner used during the build.
    ///
    /// Always available, even in failure.
    pub fn interner_mut(&mut self) -> &mut SymbolInterner {
        match self {
            Self::Success { lifted_problem, .. } => lifted_problem.interner_mut(),
            Self::Failure { interner, .. } => interner,
        }
    }

    /// Consumes and returns the string interner.
    ///
    /// Always available, even in failure.
    pub fn take_interner(&mut self) -> SymbolInterner {
        match self {
            Self::Success { lifted_problem, .. } => std::mem::take(lifted_problem.interner_mut()),
            Self::Failure { interner, .. } => std::mem::take(interner),
        }
    }

    /// Returns `true` if the IR was successfully built.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Returns `true` if the IR build failed.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure { .. })
    }
}

impl fmt::Display for Result {
    /// Formats the builder result for human-readable output.
    ///
    /// On success, displays the lifted problem and diagnostics. On failure, displays
    /// diagnostics and notes the build failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success {
                lifted_problem,
                diagnostic_manager,
            } => {
                writeln!(f, "IR built successfully:\n{}", lifted_problem)?;
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
                writeln!(f, "IR build failed.")?;
                for diag in diagnostic_manager.diagnostics() {
                    writeln!(f, "{}", diag)?;
                }
            }
        }
        Ok(())
    }
}
