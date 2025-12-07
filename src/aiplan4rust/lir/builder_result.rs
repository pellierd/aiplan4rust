//! Defines the [`LirBuilderResult`] type_checker, which encapsulates the outcome of the
//! IR (Intermediate Representation) building phase in the AI syntax pipeline.
//!
//! A [`LirBuilderResult`] contains:
//! - An optional [`LiftedProblem`] representing the successfully constructed IR of the syntax problem.
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
use crate::aiplan4rust::lir::problem::LiftedProblem;
use std::fmt;
use crate::aiplan4rust::interner::StringInterner;

/// Represents the result of the IR (Intermediate Representation) building phase.
///
/// This structure encapsulates the optional `LiftedProblem` produced during the build process,
/// as well as a `DiagnosticManager` that collects all warnings, errors, and informational
/// messages encountered.
///
/// Additionally, it may carry a `StringInterner`, allowing access to interned strings
/// even in the case of a partial or failed build. This enables consistent symbol referencing
/// and easier debugging or reporting.
///
/// # Fields
/// - `lifted_problem`: The optional result of the IR construction. `Some(LiftedProblem)` if
///   the process succeeded, otherwise `None`.
/// - `diagnostic_manager`: Captures diagnostics (errors, warnings, etc.) encountered during the build.
/// - `interner`: Optional string interner used during the build phase, retrievable even on failure.
///
/// # Use Cases
///
/// After attempting to build an IR, clients can:
/// - Inspect the result to determine whether construction succeeded.
/// - Retrieve and report diagnostics.
/// - Access or recover the `StringInterner` for consistent symbol formatting.
///
/// # Example
/// ```rust
/// let builder_result = ir_builder.build(...);
///
/// if let Some(problem) = builder_result.lifted_problem() {
///     // Use the constructed IR
/// }
///
/// for diagnostic in builder_result.diagnostic_manager().diagnostics() {
///     eprintln!("Diagnostic: {}", diagnostic);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct BuilderResult {
    lifted_problem: Option<LiftedProblem>,
    diagnostic_manager: DiagnosticManager,
    interner: Option<StringInterner>,
}

impl BuilderResult {
    /// Creates a successful [`LirBuilderResult`] with the given lifted problem
    /// and diagnostic manager.
    ///
    /// The interner is not set in this case (`None`).
    /// This constructor should be used when IR construction succeeds and
    /// you have a valid lifted problem to return.
    ///
    /// # Parameters
    /// - `lifted_problem`: The successfully constructed [`LiftedProblem`].
    /// - `diagnostic_manager`: Diagnostics generated during IR construction,
    ///   which may include warnings or notes but no fatal errors.
    ///
    /// # Returns
    /// A [`LirBuilderResult`] representing a successful IR construction,
    /// with no string interner attached.
    pub fn success(
        lifted_problem: LiftedProblem,
        diagnostic_manager: DiagnosticManager,
    ) -> Self {
        Self {
            lifted_problem: Some(lifted_problem),
            diagnostic_manager,
            interner: None,
        }
    }

    /// Creates a failure [`LirBuilderResult`] with diagnostic manager and string interner,
    /// but without a lifted problem.
    ///
    /// This constructor should be used when IR construction fails
    /// and you want to preserve diagnostics and the string interner for reporting or debugging.
    ///
    /// # Parameters
    /// - `diagnostic_manager`: Diagnostics generated during IR construction,
    ///   including errors preventing IR construction.
    /// - `interner`: The [`StringInterner`] used during IR construction,
    ///   useful for consistent symbol reporting even on failure.
    ///
    /// # Returns
    /// A [`LirBuilderResult`] representing a failed IR construction,
    /// with diagnostics and the string interner preserved, but no IR result.
    pub fn failure(
        diagnostic_manager: DiagnosticManager,
        interner: StringInterner,
    ) -> Self {
        Self {
            lifted_problem: None,
            diagnostic_manager,
            interner: Some(interner),
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

    /// Returns a reference to the `StringInterner` used during IR construction.
    ///
    /// Panics if neither the `LiftedProblem` nor the local interner is present.
    pub fn interner(&self) -> &StringInterner {
        if let Some(lifted_problem) = &self.lifted_problem {
            lifted_problem.interner()
        } else {
            self.interner
                .as_ref()
                .expect("Expected an interner to be present in BuilderResult")
        }
    }

    /// Returns a mutable reference to the `StringInterner` used during IR construction.
    ///
    /// Panics if neither the `LiftedProblem` nor the local interner is present.
    pub fn interner_mut(&mut self) -> &mut StringInterner {
        if let Some(lifted_problem) = &mut self.lifted_problem {
            lifted_problem.interner_mut()
        } else {
            self.interner
                .as_mut()
                .expect("Expected a mutable interner to be present in BuilderResult")
        }
    }

    /// Consumes and returns the `StringInterner` used during IR construction.
    ///
    /// Panics if neither the `LiftedProblem` nor the local interner is present.
    pub fn take_interner(&mut self) -> StringInterner {
        if let Some(lifted_problem) = &mut self.lifted_problem {
            std::mem::take(lifted_problem.interner_mut())
        } else {
            self.interner
                .take()
                .expect("Expected an interner to take from BuilderResult")
        }
    }

    /// Returns `true` if the LIR was successfully built.
    pub fn is_success(&self) -> bool {
        self.lifted_problem.is_some()
    }

    /// Returns `true` if no LIR was built.
    pub fn is_failure(&self) -> bool {
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
