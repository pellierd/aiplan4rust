use crate::aiplan4rust::error::ErrorManager;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;

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
    annotated_syntax_tree: Option<AnnotatedSyntaxTree>,
    error_manager: ErrorManager,
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
        annotated_syntax_tree: Option<AnnotatedSyntaxTree>,
        error_manager: ErrorManager,
    ) -> Self {
        AnalyzerResult {
            annotated_syntax_tree,
            error_manager,
        }
    }

    /// Returns an immutable reference to the annotated syntax tree.
    ///
    /// # Returns
    /// `Some(&AnnotatedSyntaxTree)` if the tree exists, otherwise `None`.
    pub fn annotated_syntax_tree(&self) -> Option<&AnnotatedSyntaxTree> {
        self.annotated_syntax_tree.as_ref()
    }

    /// Returns a mutable reference to the annotated syntax tree.
    ///
    /// # Returns
    /// `Some(&mut AnnotatedSyntaxTree)` if the tree exists, otherwise `None`.
    pub fn annotated_syntax_tree_mut(&mut self) -> Option<&mut AnnotatedSyntaxTree> {
        self.annotated_syntax_tree.as_mut()
    }

    /// Returns an immutable reference to the error manager.
    ///
    /// # Returns
    /// A reference to the `ErrorManager`.
    pub fn error_manager(&self) -> &ErrorManager {
        &self.error_manager
    }

    /// Returns a mutable reference to the error manager.
    ///
    /// # Returns
    /// A mutable reference to the `ErrorManager`.
    pub fn error_manager_mut(&mut self) -> &mut ErrorManager {
        &mut self.error_manager
    }

    /// Checks whether an annotated syntax tree is present.
    ///
    /// # Returns
    /// `true` if the annotated syntax tree exists, `false` otherwise.
    pub fn is_some(&self) -> bool {
        self.annotated_syntax_tree.is_some()
    }

    /// Checks whether an annotated syntax tree is absent.
    ///
    /// # Returns
    /// `true` if the annotated syntax tree is absent, `false` otherwise.
    pub fn is_none(&self) -> bool {
        self.annotated_syntax_tree.is_none()
    }
}

impl fmt::Display for AnalyzerResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.annotated_syntax_tree {
            Some(tree) => {
                // If the annotated syntax tree exists, display the tree and any errors.
                write!(f, "Semantic analysis successful:\n{}", tree)?;

                // Check if there are any errors in the error manager.
                if !self.error_manager.is_empty() {
                    write!(f, "\nErrors encountered during analysis:\n")?;
                    for error in self.error_manager.errors() {
                        write!(f, "{}\n", error)?;
                    }
                } else {
                    write!(f, "\nNo errors detected.")?;
                }
            }
            None => {
                // If no annotated syntax tree is available, display analysis failure and errors.
                write!(f, "Semantic analysis failed:\n")?;
                for error in self.error_manager.errors() {
                    write!(f, "{}\n", error)?;
                }
            }
        }
        Ok(())
    }
}
