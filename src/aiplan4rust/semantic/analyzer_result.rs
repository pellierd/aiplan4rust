use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::semantic::SemanticContext;

use std::fmt;


/// `AnalyzerResult` represents the result of a semantic analysis, which contains
/// an annotated syntax tree and an associated error manager.
///
/// This structure is used to store the result of the analysis, whether it's successful
/// or contains errors that need to be handled.
///
/// # Fields
/// - `annotated_syntax_tree`: An optional `AnnotatedSyntaxTree` that represents the
///   result of the semantic analysis. It will be `Some(AnnotatedSyntaxTree)` if
///   the analysis was successful, or `None` if an error occurred during the analysis.
/// - `error_manager`: The `ErrorManager` that collects any errors encountered
///   during the analysis process.
///
/// # Methods
/// - `new`: Creates a new instance of `AnalyzerResult` with an optional annotated
///   syntax tree and an error manager.
/// - `annotated_syntax_tree`: Returns an immutable reference to the annotated syntax tree,
///   or `None` if the tree is unavailable.
/// - `annotated_syntax_tree_mut`: Returns a mutable reference to the annotated syntax tree,
///   or `None` if the tree is unavailable.
/// - `error_manager`: Returns an immutable reference to the error manager.
/// - `error_manager_mut`: Returns a mutable reference to the error manager.
/// - `is_some`: Checks whether an annotated syntax tree is present.
/// - `is_none`: Checks whether an annotated syntax tree is absent.

#[derive(Debug, Clone)]
pub struct AnalyzerResult {
    context: Option<SemanticContext>,
    diagnostic_manager: DiagnosticManager,
}

impl AnalyzerResult {
    /// Creates a new `AnalyzerResult` with an optional annotated syntax tree and an error manager.
    ///
    /// # Arguments
    /// - `annotated_syntax_tree`: The annotated syntax tree associated with this analysis result.
    /// - `error_manager`: The error manager that collects all errors encountered during the analysis.
    ///
    /// # Returns
    /// An `AnalyzerResult` containing the provided values.
    pub fn new(
        context: Option<SemanticContext>,
        diagnostic_manager: DiagnosticManager,
    ) -> Self {
        AnalyzerResult {
            context,
            diagnostic_manager,
        }
    }

    /// Returns an immutable reference to the annotated syntax tree.
    ///
    /// # Returns
    /// `Some(&AnnotatedSyntaxTree)` if the tree exists, otherwise `None`.
    pub fn semantic_context(&self) -> Option<&SemanticContext> {
        self.context.as_ref()
    }

    /// Consumes the current instance and returns the annotated syntax tree if present.
    ///
    /// This function takes ownership of `self` and extracts the `AnnotatedSyntaxTree`
    /// from it, if it exists. This is useful when you need to move the syntax tree
    /// out of the structure rather than borrowing it.
    ///
    /// # Returns
    ///
    /// `Some(AnnotatedSyntaxTree)` if the syntax tree exists, or `None` otherwise.
    pub fn into_semantic_context(self) -> Option<SemanticContext> {
        self.context
    }

    /// Returns a mutable reference to the annotated syntax tree.
    ///
    /// # Returns
    /// `Some(&mut AnnotatedSyntaxTree)` if the tree exists, otherwise `None`.
    pub fn semantic_context_mut(&mut self) -> Option<&mut SemanticContext> {
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

    /// Checks whether an annotated syntax tree is present.
    ///
    /// # Returns
    /// `true` if the annotated syntax tree exists, `false` otherwise.
    pub fn is_some(&self) -> bool {
        self.context.is_some()
    }

    /// Checks whether an annotated syntax tree is absent.
    ///
    /// # Returns
    /// `true` if the annotated syntax tree is absent, `false` otherwise.
    pub fn is_none(&self) -> bool {
        self.context.is_none()
    }
}

impl fmt::Display for AnalyzerResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.context {
            Some(context) => {
                // If the annotated syntax tree exists, display the tree and any errors.
                write!(f, "Semantic analysis successful:\n{}", context)?;

                // Check if there are any errors in the error manager.
                if !self.diagnostic_manager().is_empty() {
                    write!(f, "\nErrors encountered during analysis:\n")?;
                    for diagnostic in self.diagnostic_manager().diagnostics() {
                        write!(f, "{}\n", diagnostic)?;
                    }
                } else {
                    write!(f, "\nNo errors detected.")?;
                }
            }
            None => {
                // If no annotated syntax tree is available, display analysis failure and errors.
                write!(f, "Semantic analysis failed:\n")?;
                for diagnostic in self.diagnostic_manager().diagnostics() {
                    write!(f, "{}\n", diagnostic)?;
                }
            }
        }
        Ok(())
    }
}
