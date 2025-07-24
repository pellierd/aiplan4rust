//! Defines the [`LirBuilderResult`] type, which encapsulates the outcome of the
//! IR (Intermediate Representation) building phase in the AI planning pipeline.
//!
//! A [`LirBuilderResult`] contains:
//! - An optional [`LiftedProblem`] representing the successfully constructed IR of the planning problem.
//! - A [`DiagnosticManager`] collecting diagnostics such as errors, warnings, and informational messages
//!   encountered during IR construction.
//!
//! This design enables detailed diagnostic reporting alongside partial or failed IR generation,
//! facilitating robust error handling and user feedback during parsing and compilation stages.
//!
//! # Usage
//! - Check if IR is present using [`LirBuilderResult::is_some`].
//! - Access diagnostics regardless of build success to understand issues.
//! - Use [`LirBuilderResult::lifted_problem`] to obtain the constructed IR when available.

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::lir::LiftedProblem;
use std::fmt;

/// Represents the result of the LIR building phase.
///
/// Bundles the optional constructed IR with the diagnostics collected during the process,
/// allowing callers to inspect both the output and any warnings or errors.
///
/// This separation of concerns improves robustness in the face of partial failures.
#[derive(Debug, Clone)]
pub struct BuilderResult {
    lifted_problem: Option<LiftedProblem>,
    diagnostic_manager: DiagnosticManager,
}

impl BuilderResult {
    /// Creates a new [`LirBuilderResult`] with the given IR and diagnostics.
    ///
    /// # Parameters
    /// - `lifted_problem`: `Some` if the LIR was successfully built, otherwise `None`.
    /// - `diagnostic_manager`: Diagnostics collected during IR construction.
    ///
    /// # Returns
    /// A new [`LirBuilderResult`] instance.
    pub fn new(lifted_problem: Option<LiftedProblem>, diagnostic_manager: DiagnosticManager) -> Self {
        Self {
            lifted_problem,
            diagnostic_manager,
        }
    }

    /// Returns a reference to the constructed IR, if available.
    ///
    /// # Returns
    /// - `Some(&LiftedProblem)` if LIR build succeeded.
    /// - `None` if no IR was produced.
    pub fn lifted_problem(&self) -> Option<&LiftedProblem> {
        self.lifted_problem.as_ref()
    }

    /// Returns a mutable reference to the constructed LIR, if available.
    pub fn lifted_problem_mut(&mut self) -> Option<&mut LiftedProblem> {
        self.lifted_problem.as_mut()
    }

    /// Returns a reference to the diagnostic manager.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the diagnostic manager.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }

    /// Extracts the diagnostic manager, replacing it with an empty one.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        std::mem::take(&mut self.diagnostic_manager)
    }

    /// Returns `true` if the LIR was successfully built.
    pub fn is_some(&self) -> bool {
        self.lifted_problem.is_some()
    }

    /// Returns `true` if no LIR was built.
    pub fn is_none(&self) -> bool {
        self.lifted_problem.is_none()
    }
}

impl fmt::Display for BuilderResult {
    /// Formats the builder result, showing whether IR was successfully built,
    /// along with any diagnostics collected.
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
