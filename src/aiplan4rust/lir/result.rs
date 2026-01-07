use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::interner::StringInterner;
use std::fmt;

/// Represents the outcome of the IR (Intermediate Representation) building phase.
///
/// `LirBuilderResult` encapsulates either a successfully constructed `LiftedProblem`
/// or a failure with diagnostics. It also preserves the `StringInterner` used during
/// the build, allowing consistent access to identifiers and symbols even in case of failure.
///
/// # Variants
///
/// - `Success` — IR was successfully built and contains the `LiftedProblem`
///   along with a `DiagnosticManager`.
/// - `Failure` — IR build failed and contains a `DiagnosticManager` and optionally
///   a `StringInterner` for reporting or debugging purposes.
///
/// # Example
///
/// ```rust
/// use crate::lir::builder::LirBuilderResult;
/// use crate::DiagnosticManager;
/// use crate::aiplan4rust::lir::problem::LiftedProblem;
///
/// let diag_manager = DiagnosticManager::new();
/// let lifted_problem = LiftedProblem::new_dummy(); // hypothetical constructor
///
/// let result = LirBuilderResult::success(lifted_problem, diag_manager.clone());
/// assert!(result.is_success());
///
/// let failed = LirBuilderResult::failure(diag_manager, StringInterner::new());
/// assert!(failed.is_failure());
/// ```
#[derive(Debug, Clone)]
pub enum Result {
    /// IR was successfully built.
    Success {
        /// The constructed lifted problem.
        lifted_problem: LiftedProblem,
        /// Diagnostics collected during the build.
        diagnostic_manager: DiagnosticManager,
    },
    /// IR build failed.
    Failure {
        /// Diagnostics explaining why the build failed.
        diagnostic_manager: DiagnosticManager,
        /// Optional interner preserved for consistent symbol reporting.
        interner: Option<StringInterner>,
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
    /// A `LirBuilderResult::Success` variant.
    pub fn success(
        lifted_problem: LiftedProblem,
        diagnostic_manager: DiagnosticManager,
    ) -> Self {
        Result::Success {
            lifted_problem,
            diagnostic_manager,
        }
    }

    /// Creates a failed builder result with diagnostics and optional interner.
    ///
    /// # Parameters
    /// - `diagnostic_manager` — Diagnostics explaining the failure.
    /// - `interner` — Optional interner used during the build for reporting.
    ///
    /// # Returns
    /// A `LirBuilderResult::Failure` variant.
    pub fn failure(
        diagnostic_manager: DiagnosticManager,
        interner: StringInterner,
    ) -> Self {
        Result::Failure {
            diagnostic_manager,
            interner: Some(interner),
        }
    }

    /// Returns a reference to the constructed IR if available.
    ///
    /// # Returns
    /// - `Some(&LiftedProblem)` if the build succeeded.
    /// - `None` if the build failed.
    pub fn lifted_problem(&self) -> Option<&LiftedProblem> {
        match self {
            Result::Success { lifted_problem, .. } => Some(lifted_problem),
            Result::Failure { .. } => None,
        }
    }

    /// Returns a mutable reference to the constructed IR if available.
    pub fn lifted_problem_mut(&mut self) -> Option<&mut LiftedProblem> {
        match self {
            Result::Success { lifted_problem, .. } => Some(lifted_problem),
            Result::Failure { .. } => None,
        }
    }

    /// Consumes and returns the lifted problem if available.
    pub fn take_lifted_problem(&mut self) -> Option<LiftedProblem> {
        match self {
            Result::Success { lifted_problem, .. } => Some(std::mem::take(lifted_problem)),
            Result::Failure { .. } => None,
        }
    }

    /// Returns a reference to the diagnostic manager.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        match self {
            Result::Success { diagnostic_manager, .. } => diagnostic_manager,
            Result::Failure { diagnostic_manager, .. } => diagnostic_manager,
        }
    }

    /// Returns a mutable reference to the diagnostic manager.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        match self {
            Result::Success { diagnostic_manager, .. } => diagnostic_manager,
            Result::Failure { diagnostic_manager, .. } => diagnostic_manager,
        }
    }

    /// Takes ownership of the diagnostic manager, leaving an empty one.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        match self {
            Result::Success { diagnostic_manager, .. } => std::mem::take(diagnostic_manager),
            Result::Failure { diagnostic_manager, .. } => std::mem::take(diagnostic_manager),
        }
    }

    /// Returns a reference to the string interner used during the build.
    ///
    /// Panics if neither a lifted problem nor interner is present.
    pub fn interner(&self) -> &StringInterner {
        match self {
            Result::Success { lifted_problem, .. } => lifted_problem.interner(),
            Result::Failure { interner, .. } => interner
                .as_ref()
                .expect("Expected interner in failed LIR builder result"),
        }
    }

    /// Returns a mutable reference to the string interner used during the build.
    ///
    /// Panics if neither a lifted problem nor interner is present.
    pub fn interner_mut(&mut self) -> &mut StringInterner {
        match self {
            Result::Success { lifted_problem, .. } => lifted_problem.interner_mut(),
            Result::Failure { interner, .. } => interner
                .as_mut()
                .expect("Expected interner in failed LIR builder result"),
        }
    }

    /// Consumes and returns the string interner.
    ///
    /// Panics if neither a lifted problem nor interner is present.
    pub fn take_interner(&mut self) -> StringInterner {
        match self {
            Result::Success { lifted_problem, .. } => std::mem::take(lifted_problem.interner_mut()),
            Result::Failure { interner, .. } => interner
                .take()
                .expect("Expected interner in failed LIR builder result"),
        }
    }

    /// Returns `true` if the IR was successfully built.
    pub fn is_success(&self) -> bool {
        matches!(self, Result::Success { .. })
    }

    /// Returns `true` if the IR build failed.
    pub fn is_failure(&self) -> bool {
        matches!(self, Result::Failure { .. })
    }
}

impl fmt::Display for Result {
    /// Formats the builder result for human-readable output.
    ///
    /// On success, displays the lifted problem and diagnostics. On failure, displays
    /// diagnostics and notes the build failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Result::Success { lifted_problem, diagnostic_manager } => {
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
            Result::Failure { diagnostic_manager, .. } => {
                writeln!(f, "IR build failed.")?;
                for diag in diagnostic_manager.diagnostics() {
                    writeln!(f, "{}", diag)?;
                }
            }
        }
        Ok(())
    }
}
