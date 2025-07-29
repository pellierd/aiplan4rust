//! Module providing the `AnalyzerResult` type that encapsulates the outcome of semantic analysis.
//!
//! This structure combines an optional semantically annotated syntax context (`SemanticContext`)
//! with a diagnostic manager (`DiagnosticManager`) that collects errors and warnings encountered
//! during the analysis process.
//!
//! It serves as the primary container for the result of semantic checks,
//! allowing callers to access the enriched AST or handle errors accordingly.
//!
//! # Key Concepts
//!
//! - `SemanticContext`: Represents the annotated syntax tree enriched with semantic information such as symbol tables, requirements, etc.
//! - `DiagnosticManager`: Collects and manages diagnostics (errors, warnings, notes) produced during analysis.
//!
//! # Usage
//!
//! ```rust
//! use aiplan4rust::semantic::{SemanticContext, AnalyzerResult};
//! use aiplan4rust::diagnostic::DiagnosticManager;
//!
//! // Create or obtain a SemanticContext and DiagnosticManager from analysis
//! let semantic_context = Some(SemanticContext::new(...));
//! let diagnostics = DiagnosticManager::new();
//!
//! // Construct the result
//! let result = AnalyzerResult::new(semantic_context, diagnostics);
//!
//! // Access the semantic context if available
//! if let Some(ctx) = result.semantic_context() {
//!     println!("Semantic analysis succeeded.");
//! }
//!
//! // Inspect diagnostics
//! if !result.diagnostic_manager().is_empty() {
//!     for diagnostic in result.diagnostic_manager().diagnostics() {
//!         println!("{}", diagnostic);
//!     }
//! }
//! ```
//!
//! # Error Handling
//!
//! The `AnalyzerResult` provides methods to query and mutate the contained semantic context
//! and diagnostic manager, facilitating robust error reporting and recovery strategies.

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::semantic::SemanticContext;

use std::fmt;
use crate::aiplan4rust::interner::StringInterner;

/// Represents the outcome of semantic analysis, including the annotated semantic context and diagnostic information.
///
/// This structure holds an optional [`SemanticContext`] that represents the enriched syntax tree
/// with semantic information, and a [`DiagnosticManager`] that collects errors, warnings,
/// and notes encountered during analysis.
///
/// # Fields
///
/// - `context`: Optional semantic context produced by the analysis.
/// - `diagnostic_manager`: Collects diagnostics (errors, warnings, notes) during analysis.
/// - `interner`: Optional [`StringInterner`] associated with the semantic context.
///
/// # Methods
///
/// Provides accessors and mutators for the semantic context and diagnostic manager,
/// as well as convenience methods to check presence or absence of the semantic context.
#[derive(Debug, Clone)]
pub struct AnalyzerResult {
    context: Option<SemanticContext>,
    diagnostic_manager: DiagnosticManager,
    interner: Option<StringInterner>,
}

impl AnalyzerResult {
    /// Creates a new `AnalyzerResult` instance.
    ///
    /// # Arguments
    ///
    /// - `context`: An optional [`SemanticContext`] representing the semantic analysis output.
    /// - `diagnostic_manager`: A [`DiagnosticManager`] that collects any diagnostics.
    /// - `interner`: An optional [`StringInterner`] associated with the analysis.
    ///
    /// # Returns
    ///
    /// A new `AnalyzerResult` encapsulating the analysis result and diagnostics.
    pub fn new(
        context: Option<SemanticContext>,
        diagnostic_manager: DiagnosticManager,
        interner: Option<StringInterner>,
    ) -> Self {
        AnalyzerResult {
            context,
            diagnostic_manager,
            interner,
        }
    }

    /// Returns an immutable reference to the semantic context, if present.
    ///
    /// # Returns
    /// `Some(&SemanticContext)` if available, or `None` if analysis failed.
    pub fn semantic_context(&self) -> Option<&SemanticContext> {
        self.context.as_ref()
    }

    /// Returns a mutable reference to the semantic context, if present.
    ///
    /// # Returns
    /// `Some(&mut SemanticContext)` if available, or `None` if analysis failed.
    pub fn semantic_context_mut(&mut self) -> Option<&mut SemanticContext> {
        self.context.as_mut()
    }

    /// Takes ownership of the semantic context out of the `AnalyzerResult`, leaving `None` in its place.
    ///
    /// # Returns
    /// `Some(SemanticContext)` if it was present, otherwise `None`.
    pub fn take_semantic_context(&mut self) -> Option<SemanticContext> {
        self.context.take()
    }

    /// Returns an immutable reference to the diagnostic manager.
    ///
    /// # Returns
    /// Reference to the `DiagnosticManager` collecting errors and warnings.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the diagnostic manager.
    ///
    /// # Returns
    /// Mutable reference to the `DiagnosticManager`.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }

    /// Takes ownership of the diagnostic manager out of the `AnalyzerResult`, leaving a default empty one in its place.
    ///
    /// # Returns
    /// The owned `DiagnosticManager`.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        std::mem::take(&mut self.diagnostic_manager)
    }

    /// Returns a reference to the `StringInterner` associated with the semantic context if present,
    /// otherwise returns a reference to the local interner.
    ///
    /// # Returns
    ///
    /// An `Option` containing a shared reference to the [`StringInterner`], or `None` if neither is available.
    pub fn interner(&self) -> Option<&StringInterner> {
        if let Some(context) = &self.context {
            Some(context.interner())
        } else {
            self.interner.as_ref()
        }
    }

    /// Returns a mutable reference to the `StringInterner` associated with the semantic context if present,
    /// otherwise returns a mutable reference to the local interner.
    ///
    /// # Returns
    ///
    /// An `Option` containing a mutable reference to the [`StringInterner`], or `None` if neither is available.
    pub fn interner_mut(&mut self) -> Option<&mut StringInterner> {
        if let Some(context) = &mut self.context {
            Some(context.interner_mut())
        } else {
            self.interner.as_mut()
        }
    }

    /// Consumes and returns the `StringInterner` associated with the semantic context if present,
    /// otherwise consumes and returns the local interner.
    ///
    /// This leaves `None` in place of the interner in either location.
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

    /// Returns `true` if a semantic context is present.
    pub fn is_some(&self) -> bool {
        self.context.is_some()
    }

    /// Returns `true` if no semantic context is present.
    pub fn is_none(&self) -> bool {
        self.context.is_none()
    }
}

impl fmt::Display for AnalyzerResult {
    /// Formats the analyzer result for user-friendly output.
    ///
    /// Displays the semantic context if present, followed by any diagnostics.
    /// If no semantic context is present, it reports that analysis failed and lists diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.context {
            Some(context) => {
                write!(f, "Semantic analysis successful:\n{}", context)?;

                if !self.diagnostic_manager().is_empty() {
                    write!(f, "\nDiagnostics during analysis:\n")?;
                    for diagnostic in self.diagnostic_manager().diagnostics() {
                        write!(f, "{}\n", diagnostic)?;
                    }
                } else {
                    write!(f, "\nNo diagnostics reported.")?;
                }
            }
            None => {
                write!(f, "Semantic analysis failed:\n")?;
                for diagnostic in self.diagnostic_manager().diagnostics() {
                    write!(f, "{}\n", diagnostic)?;
                }
            }
        }
        Ok(())
    }
}
