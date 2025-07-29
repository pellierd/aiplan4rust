//! Module handling the result of the semantic linking phase between domain and problem ASTs.
//!
//! This module defines the [`LinkerResult`] struct, which encapsulates the outcome
//! of the linking process that associates semantic information between a syntax domain
//! and problem representation.
//!
//! The linking process attempts to create a [`LinkedSemanticContext`] that combines
//! the domain and problem ASTs in a semantically consistent way. Alongside the linked context,
//! a [`DiagnosticManager`] collects any warnings, errors, or informational messages
//! generated during linking.
//!
//! # Typical Usage
//!
//! After performing linking, users receive a `LinkerResult` that:
//! - Contains an optional linked semantic context (present if linking succeeded).
//! - Provides access to diagnostics for troubleshooting or reporting.
//!
//! The result can be inspected to decide whether to continue further analysis or
//! to handle errors accordingly.

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::linking::LinkedSemanticContext;

use std::fmt;
use crate::aiplan4rust::interner::StringInterner;

/// Represents the result of the semantic linking process between a domain and a problem.
///
/// A `LinkerResult` encapsulates:
/// - An optional [`LinkedSemanticContext`] produced by successful linking.
/// - A [`DiagnosticManager`] containing diagnostics collected during the linking phase.
/// - An optional [`StringInterner`] used during linking, available even if linking fails.
///
/// # Fields
/// - `context`: An optional `LinkedSemanticContext` representing the unified semantic representation. `None` if linking failed.
/// - `diagnostic_manager`: Accumulates warnings, errors, and other diagnostics from the linking process.
/// - `interner`: A `StringInterner` that may contain symbols shared or unified during linking. Retained even on failure.
///
/// # Usage
/// This struct allows consumers to:
/// - Check if linking succeeded by inspecting `context`.
/// - Retrieve all diagnostics via `diagnostic_manager`.
/// - Access the symbol `interner`, regardless of success or failure.
///
/// # Example
/// ```rust
/// let linker_result = linker.link(domain_ctx, problem_ctx);
///
/// if let Some(linked_ctx) = linker_result.linked_semantic_context() {
///     // Use the linked context
/// } else {
///     // Inspect diagnostics for failure reasons
///     for diag in linker_result.diagnostic_manager().diagnostics() {
///         println!("Link error: {}", diag);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct LinkerResult {
    context: Option<LinkedSemanticContext>,
    diagnostic_manager: DiagnosticManager,
    interner: Option<StringInterner>,
}

impl LinkerResult {
    /// Creates a successful `LinkerResult` with a linked context.
    ///
    /// The interner is taken from the context automatically.
    ///
    /// # Arguments
    ///
    /// * `context` - The successfully linked semantic context.
    /// * `diagnostic_manager` - The diagnostics collected during linking.
    ///
    /// # Returns
    ///
    /// A `LinkerResult` representing a successful linking operation.
    pub fn success(context: LinkedSemanticContext, diagnostic_manager: DiagnosticManager) -> Self {
        let interner = Some(context.interner().clone());
        LinkerResult {
            context: Some(context),
            diagnostic_manager,
            interner,
        }
    }

    /// Creates a failure `LinkerResult` without a linked context.
    ///
    /// Requires explicit diagnostics and interner because no context is available.
    ///
    /// # Arguments
    ///
    /// * `diagnostic_manager` - The diagnostics collected during linking.
    /// * `interner` - The interner used during linking.
    ///
    /// # Returns
    ///
    /// A `LinkerResult` representing a failed linking operation.
    pub fn failure(diagnostic_manager: DiagnosticManager, interner: StringInterner) -> Self {
        LinkerResult {
            context: None,
            diagnostic_manager,
            interner: Some(interner),
        }
    }

    /// Returns an immutable reference to the linked semantic context, if available.
    ///
    /// # Returns
    /// `Some(&LinkedSemanticContext)` if available, `None` otherwise.
    pub fn linked_semantic_context(&self) -> Option<&LinkedSemanticContext> {
        self.context.as_ref()
    }

    /// Extracts the linked semantic context, leaving `None` in its place.
    ///
    /// # Returns
    /// `Some(LinkedSemanticContext)` if available, `None` otherwise.
    pub fn take_linked_semantic_context(&mut self) -> Option<LinkedSemanticContext> {
        self.context.take()
    }

    /// Returns a mutable reference to the linked semantic context, if available.
    ///
    /// # Returns
    /// `Some(&mut LinkedSemanticContext)` if available, `None` otherwise.
    pub fn linked_semantic_context_mut(&mut self) -> Option<&mut LinkedSemanticContext> {
        self.context.as_mut()
    }

    /// Returns an immutable reference to the diagnostic manager.
    ///
    /// # Returns
    /// A reference to the `DiagnosticManager` containing diagnostics from linking.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the diagnostic manager.
    ///
    /// # Returns
    /// A mutable reference to the `DiagnosticManager`.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }

    /// Extracts the diagnostic manager, replacing it with an empty one.
    ///
    /// # Returns
    /// The owned `DiagnosticManager` with all diagnostics collected during linking.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        std::mem::take(&mut self.diagnostic_manager)
    }

    /// Returns a reference to the `StringInterner` used during linking.
    ///
    /// If the `LinkedSemanticContext` is present, returns a reference to its internal interner.
    /// Otherwise, falls back to the local interner stored in the `LinkerResult`.
    ///
    /// # Returns
    ///
    /// `Some(&StringInterner)` if an interner is available, or `None` if both the context
    /// and local interner are absent.
    pub fn interner(&self) -> Option<&StringInterner> {
        if let Some(context) = &self.context {
            Some(context.interner())
        } else {
            self.interner.as_ref()
        }
    }

    /// Returns a mutable reference to the `StringInterner` used during linking.
    ///
    /// If the `LinkedSemanticContext` is present, returns a mutable reference to its internal interner.
    /// Otherwise, returns a mutable reference to the local interner stored in the `LinkerResult`.
    ///
    /// # Returns
    ///
    /// `Some(&mut StringInterner)` if an interner is available, or `None` if both the context
    /// and local interner are absent.
    pub fn interner_mut(&mut self) -> Option<&mut StringInterner> {
        if let Some(context) = &mut self.context {
            Some(context.interner_mut())
        } else {
            self.interner.as_mut()
        }
    }

    /// Consumes and returns the `StringInterner` used during linking.
    ///
    /// If the `LinkedSemanticContext` is present, the interner is taken from it using `mem::take`.
    /// Otherwise, it is taken from the local interner field of the `LinkerResult`.
    ///
    /// This operation leaves `None` in place of the interner (either in the context or locally),
    /// effectively transferring ownership.
    ///
    /// # Returns
    ///
    /// An `Option<StringInterner>` containing the taken interner, or `None` if neither is present.
    pub fn take_interner(&mut self) -> Option<StringInterner> {
        if let Some(context) = &mut self.context {
            Some(std::mem::take(context.interner_mut()))
        } else {
            self.interner.take()
        }
    }

    /// Returns `true` if the linking produced a semantic context (`Some`).
    ///
    /// # Returns
    /// `true` if linking succeeded and produced a context, `false` otherwise.
    pub fn is_success(&self) -> bool {
        self.context.is_some()
    }

    /// Returns `true` if the linking did not produce a semantic context (`None`).
    ///
    /// # Returns
    /// `true` if linking failed or produced no context, `false` otherwise.
    pub fn is_failure(&self) -> bool {
        self.context.is_none()
    }
}

impl fmt::Display for LinkerResult {
    /// Formats the `LinkerResult` for display.
    ///
    /// This method prints a summary of the linking process:
    /// - If linking succeeded (i.e., context is `Some`), it displays the linked context
    ///   and any diagnostics collected during linking.
    /// - If linking failed (i.e., context is `None`), it prints the failure message
    ///   and all diagnostics.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure of the write operation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.context {
            Some(context) => {
                write!(f, "Linking successful:\n{}", context)?;

                if !self.diagnostic_manager().is_empty() {
                    write!(f, "\nDiagnostics:\n")?;
                    for diagnostic in self.diagnostic_manager().diagnostics() {
                        writeln!(f, "{}", diagnostic)?;
                    }
                } else {
                    writeln!(f, "\nNo diagnostics reported.")?;
                }
            }
            None => {
                writeln!(f, "Linking failed:\n")?;
                for diagnostic in self.diagnostic_manager().diagnostics() {
                    writeln!(f, "{}", diagnostic)?;
                }
            }
        }
        Ok(())
    }
}
