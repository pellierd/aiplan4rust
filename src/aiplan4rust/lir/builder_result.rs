//! Module defining the `BuilderResult` type,
//! which represents the result of the IR building phase in the AI planning framework.
//!
//! The `BuilderResult` contains an optional `LiftedProblem` representing the constructed
//! intermediate representation (IR) of the planning problem, along with a `DiagnosticManager`
//! that holds any diagnostics such as errors, warnings, or informational messages encountered
//! during the build process.
//!
//! This design allows returning both the result of the build phase and any relevant diagnostics,
//! enabling better error reporting and handling in the parsing and IR building pipeline.

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::lir::LiftedProblem;
use std::fmt;

/// `BuilderResult` represents the outcome of the IR building step.
///
/// It contains an optional `LiftedProblem` which is the IR produced if the build
/// was successful, as well as a `DiagnosticManager` that accumulates diagnostics
/// (errors, warnings, info) produced during the process.
///
/// This allows to convey partial success (with diagnostics) or failure (without IR)
/// while preserving detailed feedback for the user or caller.
#[derive(Debug, Clone)]
pub struct BuilderResult {
    lifted_problem: Option<LiftedProblem>,
    diagnostic_manager: DiagnosticManager,
}

impl BuilderResult {
    /// Creates a new `BuilderResult`.
    ///
    /// # Arguments
    ///
    /// * `lifted_problem` - Optional `LiftedProblem` produced by the IR builder.
    /// * `diagnostic_manager` - The diagnostics collected during the build.
    ///
    /// # Returns
    ///
    /// A new `BuilderResult` containing the provided IR and diagnostics.
    pub fn new(lifted_problem: Option<LiftedProblem>, diagnostic_manager: DiagnosticManager) -> Self {
        Self {
            lifted_problem,
            diagnostic_manager,
        }
    }

    /// Returns a reference to the `LiftedProblem` if present.
    ///
    /// # Returns
    ///
    /// * `Some(&LiftedProblem)` if the IR was successfully built.
    /// * `None` otherwise.
    pub fn lifted_problem(&self) -> Option<&LiftedProblem> {
        self.lifted_problem.as_ref()
    }

    /// Returns a mutable reference to the `LiftedProblem` if present.
    ///
    /// # Returns
    ///
    /// * `Some(&mut LiftedProblem)` if the IR was successfully built.
    /// * `None` otherwise.
    pub fn lifted_problem_mut(&mut self) -> Option<&mut LiftedProblem> {
        self.lifted_problem.as_mut()
    }

    /// Returns a reference to the `DiagnosticManager`.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the `DiagnosticManager`.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }

    /// Returns `true` if a `LiftedProblem` is present.
    pub fn is_some(&self) -> bool {
        self.lifted_problem.is_some()
    }

    /// Returns `true` if no `LiftedProblem` is present.
    pub fn is_none(&self) -> bool {
        self.lifted_problem.is_none()
    }
}

impl fmt::Display for BuilderResult {
    /// Formats the `BuilderResult` for user-friendly display.
    ///
    /// If the IR was successfully built, it prints the IR followed by any diagnostics.
    /// Otherwise, it prints the diagnostics indicating failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.lifted_problem {
            Some(ir) => {
                write!(f, "IR built successfully:\n{}", ir)?;
                if !self.diagnostic_manager().is_empty() {
                    write!(f, "\nDiagnostics:\n")?;
                    for diagnostic in self.diagnostic_manager().diagnostics() {
                        write!(f, "{}\n", diagnostic)?;
                    }
                } else {
                    write!(f, "\nNo diagnostics.")?;
                }
            }
            None => {
                write!(f, "IR build failed:\n")?;
                for diagnostic in self.diagnostic_manager().diagnostics() {
                    write!(f, "{}\n", diagnostic)?;
                }
            }
        }
        Ok(())
    }
}
