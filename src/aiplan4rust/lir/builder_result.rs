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
use crate::aiplan4rust::lir::LiftedProblem;
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
    /// Creates a new [`LirBuilderResult`] with the given lifted IR, diagnostics, and optional string interner.
    ///
    /// # Parameters
    /// - `lifted_problem`: An optional [`LiftedProblem`] representing the result of the IR construction.
    ///   If IR construction succeeded, this will be `Some(...)`, otherwise `None`.
    /// - `diagnostic_manager`: A [`DiagnosticManager`] instance holding all diagnostics (errors, warnings, notes)
    ///   generated during the IR construction phase.
    /// - `interner`: An optional [`StringInterner`] used during IR construction. This interner can be retrieved
    ///   even in case of failure, to support consistent symbol reporting or debugging.
    ///
    /// # Returns
    /// A new [`LirBuilderResult`] instance containing the IR result, diagnostics, and interner.
    pub fn new(
        lifted_problem: Option<LiftedProblem>,
        diagnostic_manager: DiagnosticManager,
        interner: Option<StringInterner>,
    ) -> Self {
        Self {
            lifted_problem,
            diagnostic_manager,
            interner,
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
    /// If the [`LiftedProblem`] is present, this returns a reference to its internal interner.
    /// Otherwise, it falls back to the local interner stored in the [`BuilderResult`].
    ///
    /// # Returns
    ///
    /// `Some(&StringInterner)` if an interner is available, or `None` if both the IR and local interner are absent.
    pub fn interner(&self) -> Option<&StringInterner> {
        if let Some(lifted_problem) = &self.lifted_problem {
            Some(lifted_problem.interner())
        } else {
            self.interner.as_ref()
        }
    }

    /// Returns a mutable reference to the `StringInterner` used during IR construction.
    ///
    /// If the [`LiftedProblem`] is present, this returns a mutable reference to its internal interner.
    /// Otherwise, it returns a mutable reference to the local interner stored in the [`BuilderResult`].
    ///
    /// # Returns
    ///
    /// `Some(&mut StringInterner)` if an interner is available, or `None` if both the IR and local interner are absent.
    pub fn interner_mut(&mut self) -> Option<&mut StringInterner> {
        if let Some(lifted_problem) = &mut self.lifted_problem {
            Some(lifted_problem.interner_mut())
        } else {
            self.interner.as_mut()
        }
    }

    /// Consumes and returns the `StringInterner` used during IR construction.
    ///
    /// If the [`LiftedProblem`] is present, the interner is taken from it using [`std::mem::take`].
    /// Otherwise, it is taken from the local interner field of the [`BuilderResult`].
    ///
    /// This operation sets the corresponding interner field to `None`.
    ///
    /// # Returns
    ///
    /// An `Option<StringInterner>` containing the taken interner, or `None` if neither is present.
    pub fn take_interner(&mut self) -> Option<StringInterner> {
        if let Some(lifted_problem) = &mut self.lifted_problem {
            Some(std::mem::take(lifted_problem.interner_mut()))
        } else {
            self.interner.take()
        }
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
