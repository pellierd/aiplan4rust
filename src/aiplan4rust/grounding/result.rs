use crate::aiplan4rust::grounding::problem::Problem;
use crate::DiagnosticManager;

/// Represents the result of grounding a lifted planning problem.
///
/// The `GroundingResult` encapsulates both the optionally grounded problem
/// (`Problem`) and the associated diagnostics collected during the grounding process.
/// It distinguishes between successful and failed grounding operations, allowing
/// consumers to inspect both outcomes and diagnostics.
///
/// # Fields
///
/// * `problem` - `Option<Problem>` containing the grounded problem if grounding succeeded.
/// * `diagnostic_manager` - A `DiagnosticManager` storing warnings, errors, or informational messages
///   generated during grounding.
///
/// # Usage
///
/// ```rust
/// let grounded_result = GroundingResult::success(grounded_problem, diag_manager);
/// if grounded_result.is_success() {
///     println!("Grounding succeeded.");
/// } else {
///     println!("Grounding failed with diagnostics:");
///     for diag in grounded_result.diagnostic_manager().diagnostics() {
///         println!("{}", diag);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Result {
    problem: Option<Problem>,
    diagnostic_manager: DiagnosticManager,
}

impl Result {
    /// Creates a successful grounding result containing a grounded problem and diagnostics.
    ///
    /// # Arguments
    /// * `grounded_problem` - The successfully grounded problem.
    /// * `diagnostic_manager` - Diagnostics collected during the grounding process.
    ///
    /// # Returns
    /// A `GroundingResult` representing success.
    pub fn success(
        grounded_problem: Problem,
        diagnostic_manager: DiagnosticManager,
    ) -> Self {
        Self {
            problem: Some(grounded_problem),
            diagnostic_manager,
        }
    }

    /// Creates a failed grounding result containing diagnostics but no grounded problem.
    ///
    /// # Arguments
    /// * `diagnostic_manager` - Diagnostics explaining why grounding failed.
    ///
    /// # Returns
    /// A `GroundingResult` representing failure.
    pub fn failure(diagnostic_manager: DiagnosticManager) -> Self {
        Self {
            problem: None,
            diagnostic_manager,
        }
    }

    /// Returns a reference to the grounded problem if available.
    pub fn problem(&self) -> Option<&Problem> {
        self.problem.as_ref()
    }

    /// Returns a mutable reference to the grounded problem if available.
    pub fn problem_mut(&mut self) -> Option<&mut Problem> {
        self.problem.as_mut()
    }

    /// Returns a reference to the diagnostic manager.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the diagnostic manager.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }

    /// Returns `true` if grounding succeeded (i.e., a grounded problem exists).
    pub fn is_success(&self) -> bool {
        self.problem.is_some()
    }

    /// Returns `true` if grounding failed (i.e., no grounded problem exists).
    pub fn is_failure(&self) -> bool {
        self.problem.is_none()
    }
}

impl std::fmt::Display for Result {
    /// Formats the grounding result for human-readable output.
    ///
    /// On success, it prints the grounded problem and any collected diagnostics.
    /// On failure, it prints the failure message along with diagnostics.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.problem {
            Some(problem) => {
                writeln!(f, "Grounding succeeded:\n{}", problem)?;
                if !self.diagnostic_manager().is_empty() {
                    writeln!(f, "\nDiagnostics:")?;
                    for diag in self.diagnostic_manager().diagnostics() {
                        writeln!(f, "{}", diag)?;
                    }
                } else {
                    writeln!(f, "\nNo diagnostics reported.")?;
                }
            }
            None => {
                writeln!(f, "Grounding failed.")?;
                for diag in self.diagnostic_manager().diagnostics() {
                    writeln!(f, "{}", diag)?;
                }
            }
        }
        Ok(())
    }
}
