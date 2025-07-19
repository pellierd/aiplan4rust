use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::linking::LinkedSemanticContext;

use std::fmt;

/// Represents the result of the semantic linking process between a domain and a problem.
///
/// A `LinkerResult` encapsulates:
/// - An optional `LinkedSemanticContext`, produced by successful linking.
/// - A `DiagnosticManager` containing diagnostics and errors encountered during linking.
///
/// # Fields
/// - `context`: An optional `LinkedSemanticContext` produced by the linker. `None` if linking failed.
/// - `diagnostic_manager`: The `DiagnosticManager` that collected diagnostics during the linking.
///
/// # Use Cases
/// Use this struct to inspect the result of linking, retrieve diagnostics,
/// or check if linking succeeded (`is_some`) or failed (`is_none`).
#[derive(Debug, Clone)]
pub struct LinkerResult {
    context: Option<LinkedSemanticContext>,
    diagnostic_manager: DiagnosticManager,
}

impl LinkerResult {
    /// Creates a new `LinkerResult`.
    ///
    /// # Arguments
    /// - `planning_task`: The result of the linking process (`Some` if successful, `None` if not).
    /// - `diagnostic_manager`: The `DiagnosticManager` holding all diagnostics from the linking phase.
    ///
    /// # Returns
    /// A new instance of `LinkerResult`.
    pub fn new(planning_task: Option<LinkedSemanticContext>, diagnostic_manager: DiagnosticManager) -> Self {
        LinkerResult {
            context: planning_task,
            diagnostic_manager,
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

    /// Returns `true` if the linking produced a semantic context (`Some`).
    ///
    /// # Returns
    /// `true` if linking succeeded and produced a context, `false` otherwise.
    pub fn is_some(&self) -> bool {
        self.context.is_some()
    }

    /// Returns `true` if the linking did not produce a semantic context (`None`).
    ///
    /// # Returns
    /// `true` if linking failed or produced no context, `false` otherwise.
    pub fn is_none(&self) -> bool {
        self.context.is_none()
    }
}

impl fmt::Display for LinkerResult {
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
