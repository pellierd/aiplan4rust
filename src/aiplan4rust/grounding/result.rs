use crate::aiplan4rust::grounding::problem::Problem;
use crate::DiagnosticManager;
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
/// - `Failure` — Grounding failed, containing only a `DiagnosticManager`
///   explaining the errors or warnings encountered.
///
/// # Examples
///
/// ```rust
/// use crate::grounding::{GroundingResult, Problem};
/// use crate::DiagnosticManager;
///
/// let diag_manager = DiagnosticManager::new();
/// let problem = Problem::new_dummy(); // hypothetical constructor
///
/// // Success case
/// let result = GroundingResult::success(problem, diag_manager.clone());
/// assert!(result.is_success());
///
/// // Failure case
/// let failed = GroundingResult::failure(diag_manager);
/// assert!(failed.is_failure());
/// ```
#[derive(Debug, Clone)]
pub enum Result {
    /// Grounding succeeded, containing the grounded problem and diagnostics.
    Success {
        /// The grounded problem generated from the lifted problem.
        problem: Problem,
        /// Diagnostics collected during grounding (warnings, info, etc.).
        diagnostic_manager: DiagnosticManager,
    },
    /// Grounding failed, containing diagnostics but no grounded problem.
    Failure {
        /// Diagnostics explaining why grounding failed.
        diagnostic_manager: DiagnosticManager,
    },
}

impl Result {
    /// Creates a successful grounding result.
    ///
    /// # Parameters
    /// - `problem` — The grounded `Problem`.
    /// - `diagnostic_manager` — Diagnostics collected during grounding.
    ///
    /// # Returns
    /// A `GroundingResult::Success` containing the problem and diagnostics.
    pub fn success(problem: Problem, diagnostic_manager: DiagnosticManager) -> Self {
        Self::Success {
            problem,
            diagnostic_manager,
        }
    }

    /// Creates a failed grounding result.
    ///
    /// # Parameters
    /// - `diagnostic_manager` — Diagnostics explaining the failure.
    ///
    /// # Returns
    /// A `GroundingResult::Failure` containing the diagnostics.
    pub fn failure(diagnostic_manager: DiagnosticManager) -> Self {
        Self::Failure { diagnostic_manager }
    }

    /// Returns a reference to the grounded problem if successful.
    ///
    /// # Returns
    /// - `Some(&Problem)` if grounding succeeded.
    /// - `None` if grounding failed.
    pub fn problem(&self) -> Option<&Problem> {
        match self {
            Self::Success { problem, .. } => Some(problem),
            Self::Failure { .. } => None,
        }
    }

    /// Returns a mutable reference to the grounded problem if successful.
    ///
    /// # Returns
    /// - `Some(&mut Problem)` if grounding succeeded.
    /// - `None` if grounding failed.
    pub fn problem_mut(&mut self) -> Option<&mut Problem> {
        match self {
            Self::Success { problem, .. } => Some(problem),
            Self::Failure { .. } => None,
        }
    }

    /// Returns a reference to the diagnostic manager.
    ///
    /// # Returns
    /// A reference to the `DiagnosticManager` regardless of success or failure.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        match self {
            Self::Success { diagnostic_manager, .. } => diagnostic_manager,
            Self::Failure { diagnostic_manager } => diagnostic_manager,
        }
    }

    /// Returns a mutable reference to the diagnostic manager.
    ///
    /// # Returns
    /// A mutable reference to the `DiagnosticManager` regardless of success or failure.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        match self {
            Self::Success { diagnostic_manager, .. } => diagnostic_manager,
            Self::Failure { diagnostic_manager } => diagnostic_manager,
        }
    }

    /// Returns `true` if grounding succeeded.
    ///
    /// # Returns
    /// `true` if the variant is `Success`, `false` otherwise.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Returns `true` if grounding failed.
    ///
    /// # Returns
    /// `true` if the variant is `Failure`, `false` otherwise.
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure { .. })
    }
}

impl fmt::Display for Result {
    /// Formats the grounding result for human-readable output.
    ///
    /// On success, prints the grounded problem and any diagnostics.
    /// On failure, prints a failure message along with diagnostics.
    ///
    /// # Parameters
    /// - `f` — A formatter for writing the output.
    ///
    /// # Returns
    /// - `fmt::Result` indicating success or failure of the write operations.
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
            Self::Failure { diagnostic_manager } => {
                writeln!(f, "Grounding failed.")?;
                for diag in diagnostic_manager.diagnostics() {
                    writeln!(f, "{}", diag)?;
                }
            }
        }
        Ok(())
    }
}
