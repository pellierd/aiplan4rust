//! Defines the `CheckerContext` enum representing different stages or contexts
//! in the compilation or analysis pipeline.
//!
//! This context is used to categorize diagnostics and errors by the phase
//! in which they occur, allowing better filtering, reporting, and handling.
//!
//! The enum can be converted into a `DiagnosticSource` to unify reporting systems.

use crate::aiplan4rust::diagnostic::DiagnosticSource;

/// Represents the context or stage in the compilation/analysis pipeline where
/// a diagnostic or error originates.
///
/// This enum is used to tag diagnostics with the phase that produced them,
/// facilitating more precise filtering and handling in diagnostic management.
///
/// # Variants
/// - `Parser`: Diagnostics emitted during parsing phase.
/// - `SemanticAnalyzer`: Diagnostics emitted during semantic analysis phase.
/// - `Linker`: Diagnostics emitted during linking phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckerContext {
    /// Parsing stage, typically when converting source code into an AST.
    Parser,
    /// Semantic analysis stage, where meaning and type checking occur.
    SemanticAnalyzer,
    /// Linking stage, where separate compilation units or modules are linked.
    Linker,
}

impl From<CheckerContext> for DiagnosticSource {
    /// Converts a `CheckerContext` into a corresponding `DiagnosticSource`.
    ///
    /// This allows diagnostics tagged with `CheckerContext` to be easily
    /// mapped into the broader diagnostic reporting framework.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::checker_context::CheckerContext;
    /// use crate::aiplan4rust::diagnostic::DiagnosticSource;
    ///
    /// let ctx = CheckerContext::Parser;
    /// let source: DiagnosticSource = ctx.into();
    /// assert_eq!(source, DiagnosticSource::Parser);
    /// ```
    fn from(ctx: CheckerContext) -> Self {
        match ctx {
            CheckerContext::Parser => DiagnosticSource::Parser,
            CheckerContext::SemanticAnalyzer => DiagnosticSource::SemanticAnalyzer,
            CheckerContext::Linker => DiagnosticSource::Linker,
        }
    }
}
