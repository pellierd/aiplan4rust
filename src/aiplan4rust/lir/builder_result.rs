//! Defines the [`BuilderResult`] type, which encapsulates the result of the IR (Intermediate Representation)
//! building phase in the AI planning pipeline.
//!
//! A [`BuilderResult`] contains:
//! - An optional [`LiftedProblem`] representing the constructed IR of the planning problem.
//! - A [`DiagnosticManager`] collecting diagnostics such as errors, warnings, and informational messages
//!   that occurred during IR construction.
//!
//! This design supports detailed diagnostic reporting alongside partial or failed IR generation,
//! enabling robust error handling in parsing and compilation stages.

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::lir::LiftedProblem;
use std::fmt;

/// Represents the result of the IR (Intermediate Representation) building phase.
///
/// This structure bundles the outcome of the build:
/// - An optional [`LiftedProblem`] if IR construction was successful.
/// - A [`DiagnosticManager`] containing diagnostics emitted during the process.
///
/// This design allows the caller to inspect whether the IR was generated,
/// and still retrieve relevant diagnostics even in the case of failure.
#[derive(Debug, Clone)]
pub struct BuilderResult {
    lifted_problem: Option<LiftedProblem>,
    diagnostic_manager: DiagnosticManager,
}

impl BuilderResult {
    /// Constructs a new [`BuilderResult`] with the given IR and diagnostics.
    ///
    /// # Parameters
    /// - `lifted_problem`: The IR result (`Some`) if successfully built, otherwise `None`.
    /// - `diagnostic_manager`: The diagnostics collected during IR construction.
    ///
    /// # Returns
    /// A new instance of [`BuilderResult`].
    pub fn new(lifted_problem: Option<LiftedProblem>, diagnostic_manager: DiagnosticManager) -> Self {
        Self {
            lifted_problem,
            diagnostic_manager,
        }
    }

    /// Returns an immutable reference to the built [`LiftedProblem`], if available.
    ///
    /// # Returns
    /// - `Some(&LiftedProblem)` if IR is present.
    /// - `None` if IR was not successfully built.
    pub fn lifted_problem(&self) -> Option<&LiftedProblem> {
        self.lifted_problem.as_ref()
    }

    /// Returns a mutable reference to the built [`LiftedProblem`], if available.
    ///
    /// # Returns
    /// - `Some(&mut LiftedProblem)` if IR is present.
    /// - `None` otherwise.
    pub fn lifted_problem_mut(&mut self) -> Option<&mut LiftedProblem> {
        self.lifted_problem.as_mut()
    }

    /// Returns an immutable reference to the [`DiagnosticManager`].
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the [`DiagnosticManager`].
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }

    /// Extracts the diagnostic manager, replacing it with an empty one.
    ///
    /// # Returns
    /// The collected [`DiagnosticManager`] containing all diagnostics.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        std::mem::take(&mut self.diagnostic_manager)
    }

    /// Returns `true` if the IR was successfully built.
    pub fn is_some(&self) -> bool {
        self.lifted_problem.is_some()
    }

    /// Returns `true` if the IR is absent (i.e., build failed).
    pub fn is_none(&self) -> bool {
        self.lifted_problem.is_none()
    }
}

impl fmt::Display for BuilderResult {
    /// Formats the result for display, including the IR (if any) and diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.lifted_problem {
            Some(ir) => {
                writeln!(f, "IR built successfully:\n{}", ir)?;
                if !self.diagnostic_manager().is_empty() {
                    writeln!(f, "\nDiagnostics:")?;
                    for diagnostic in self.diagnostic_manager().diagnostics() {
                        writeln!(f, "{}", diagnostic)?;
                    }
                } else {
                    writeln!(f, "\nNo diagnostics reported.")?;
                }
            }
            None => {
                writeln!(f, "IR build failed.")?;
                for diagnostic in self.diagnostic_manager().diagnostics() {
                    writeln!(f, "{}", diagnostic)?;
                }
            }
        }
        Ok(())
    }
}
