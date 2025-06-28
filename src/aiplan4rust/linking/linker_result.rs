use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::linking::LinkedSemanticContext;

use std::fmt;

/// `LinkerResult` represents the result of a linking process, which contains a lifted planning
/// task and an associated error manager.
///
/// This structure is used to store the result of the linking process, whether it's successful
/// or contains errors that need to be handled.
///
/// # Fields
/// - `planning_task`: An optional `LiftedPlanningTask` that represents the result of the linking.
///   It will be `Some(LiftedPlanningTask)` if the linking was successful, or `None` if an error
///   occurred.
/// - `error_manager`: The `ErrorManager` that collects any errors encountered during the linking
///   process.
///
/// # Methods
/// - `new`: Creates a new instance of `LinkerResult` with an optional lifted planning task and an
///   error manager.
/// - `planning_task`: Returns an immutable reference to the lifted planning task, or `None` if the
///   task is unavailable.
/// - `planning_task_mut`: Returns a mutable reference to the lifted planning task, or `None` if the
///   task is unavailable.
/// - `error_manager`: Returns an immutable reference to the error manager.
/// - `error_manager_mut`: Returns a mutable reference to the error manager.
/// - `is_some`: Checks whether a lifted planning task is present.
/// - `is_none`: Checks whether a lifted planning task is absent.
#[derive(Debug, Clone)]
pub struct LinkerResult {
    context: Option<LinkedSemanticContext>,
    diagnostic_manager: DiagnosticManager,
}

impl LinkerResult {
    /// Creates a new `LinkerResult` with an optional lifted planning task and an error manager.
    ///
    /// # Arguments
    /// - `planning_task`: The lifted planning task associated with this linking result.
    /// - `error_manager`: The error manager that collects all errors encountered during the linking.
    ///
    /// # Returns
    /// A `LinkerResult` containing the provided values.
    pub fn new(planning_task: Option<LinkedSemanticContext>, diagnostic_manager: DiagnosticManager) -> Self {
        LinkerResult {
            context: planning_task,
            diagnostic_manager,
        }
    }

    /// Returns an immutable reference to the lifted planning task.
    ///
    /// # Returns
    /// `Some(&LiftedPlanningTask)` if the task exists, otherwise `None`.
    pub fn linked_semantic_context(&self) -> Option<&LinkedSemanticContext> {
        self.context.as_ref()
    }

    /// Returns a mutable reference to the lifted planning task.
    ///
    /// # Returns
    /// `Some(&mut LiftedPlanningTask)` if the task exists, otherwise `None`.
    pub fn linked_semantic_context_mut(&mut self) -> Option<&mut LinkedSemanticContext> {
        self.context.as_mut()
    }

    /// Returns an immutable reference to the error manager.
    ///
    /// # Returns
    /// A reference to the `ErrorManager`.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the error manager.
    ///
    /// # Returns
    /// A mutable reference to the `ErrorManager`.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }

    /// Checks whether a lifted planning task is present.
    ///
    /// # Returns
    /// `true` if the lifted planning task exists, `false` otherwise.
    pub fn is_some(&self) -> bool {
        self.context.is_some()
    }

    /// Checks whether a lifted planning task is absent.
    ///
    /// # Returns
    /// `true` if the lifted planning task is absent, `false` otherwise.
    pub fn is_none(&self) -> bool {
        self.context.is_none()
    }
}

impl fmt::Display for LinkerResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.context {
            Some(task) => {
                // If the lifted planning task exists, display the task and any errors.
                write!(f, "Linking successful:\n{}", task)?;

                // Check if there are any errors in the error manager.
                if !self.diagnostic_manager().is_empty() {
                    write!(f, "\nErrors encountered during linking:\n")?;
                    for diagnostic in self.diagnostic_manager().diagnostics() {
                        write!(f, "{}\n", diagnostic)?;
                    }
                } else {
                    write!(f, "\nNo errors detected.")?;
                }
            }
            None => {
                // If no lifted planning task is available, display linking failure and errors.
                write!(f, "Linking failed:\n")?;
                for diagnostic in self.diagnostic_manager().diagnostics() {
                    write!(f, "{}\n", diagnostic)?;
                }
            }
        }
        Ok(())
    }
}
