//! Module for AST normalization results and diagnostics management.
//!
//! This module defines the [`NormalizerResult`] struct, which encapsulates
//! the outcome of an AST normalization phase in the `aiplan4rust` pipeline.
//!
//! # Purpose
//!
//! `NormalizerResult` bundles together:
//! - The normalized [`Ast`] if normalization was successful.
//! - A [`DiagnosticManager`] that collects warnings, errors, and info messages
//!   generated during normalization.
//!
//! This allows users to proceed with semantic analysis only if normalization
//! succeeded, while also accessing any diagnostics that arose.
//!
//! # Usage
//!
//! After parsing and normalization, the `NormalizerResult` instance provides access
//! to the normalized AST and the diagnostics to handle errors or warnings.
//!
//! # Example
//!
//! ```rust
//! let result: NormalizerResult = normalizer.normalize(ast)?;
//!
//! if let Some(normalized_ast) = result.ast() {
//!     // Proceed with normalized AST
//! }
//!
//! for diagnostic in result.diagnostic_manager().diagnostics() {
//!     println!("Diagnostic: {}", diagnostic);
//! }
//! ```

use std::fmt;

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::syntax::ast::Ast;

/// Represents the result of the AST normalization phase.
///
/// This struct encapsulates both the normalized [`Ast`] (if normalization succeeded)
/// and the [`DiagnosticManager`] which collects all warnings, errors,
/// or informational messages generated during normalization.
///
/// The `NormalizerResult` is typically produced after parsing and normalization,
/// but before semantic analysis.
///
/// # Structure
///
/// - `ast`: An optional normalized AST. This is `Some(ast)` if normalization
///   was successful, otherwise `None`.
/// - `diagnostic_manager`: Holds diagnostics produced during normalization.
/// - `interner`: An optional [`StringInterner`] associated with the normalized AST.
///
/// # Usage
///
/// After normalization, users can inspect the normalized AST and any
/// diagnostics to determine if further processing should proceed.
///
/// ```rust
/// let result: NormalizerResult = normalizer.normalize(ast)?;
///
/// if let Some(normalized_ast) = result.ast() {
///     // Use normalized AST
/// }
///
/// for diag in result.diagnostic_manager().diagnostics() {
///     println!("Diagnostic: {}", diag);
/// }
/// ```
#[derive(Debug)]
pub struct NormalizerResult {
    ast: Option<Ast>,
    diagnostic_manager: DiagnosticManager,
    interner: Option<StringInterner>,
}

impl NormalizerResult {
    /// Creates a new `NormalizerResult` representing a successful normalization.
    ///
    /// # Parameters
    ///
    /// * `ast` - The normalized AST produced by the normalization process.
    /// * `diagnostic_manager` - The diagnostic manager capturing any diagnostics during normalization.
    ///
    /// # Returns
    ///
    /// A `NormalizerResult` instance indicating success with the normalized AST and no interner.
    ///
    /// # Behavior
    ///
    /// In a success scenario, the `interner` field is always `None`.
    pub fn success(
        ast: Ast,
        diagnostic_manager: DiagnosticManager,
    ) -> Self {
        Self {
            ast: Some(ast),
            diagnostic_manager,
            interner: None,
        }
    }

    /// Creates a new `NormalizerResult` representing a failure in normalization.
    ///
    /// # Parameters
    ///
    /// * `diagnostic_manager` - The diagnostic manager capturing errors and warnings.
    /// * `interner` - The `StringInterner` associated with the normalization process, useful for error reporting or diagnostics.
    ///
    /// # Returns
    ///
    /// A `NormalizerResult` instance indicating failure, with no AST and the given interner.
    ///
    /// # Behavior
    ///
    /// In a failure scenario, the `ast` field is always `None` and `interner` is set.
    pub fn failure(
        diagnostic_manager: DiagnosticManager,
        interner: StringInterner,
    ) -> Self {
        Self {
            ast: None,
            diagnostic_manager,
            interner: Some(interner),
        }
    }

    /// Returns an immutable reference to the normalized AST.
    ///
    /// # Returns
    ///
    /// A reference to the optional normalized AST. If normalization failed,
    /// this will be `None`.
    pub fn ast(&self) -> &Option<Ast> {
        &self.ast
    }

    /// Returns a mutable reference to the normalized AST.
    ///
    /// This allows modification or replacement of the AST within the result.
    ///
    /// # Returns
    ///
    /// A mutable reference to the optional normalized AST.
    pub fn ast_mut(&mut self) -> &mut Option<Ast> {
        &mut self.ast
    }

    /// Takes ownership of the normalized AST, leaving `None` in its place.
    ///
    /// This is useful when transferring ownership out of the result.
    ///
    /// # Returns
    ///
    /// The normalized AST if present, or `None`.
    pub fn take_ast(&mut self) -> Option<Ast> {
        self.ast.take()
    }

    /// Takes ownership of the diagnostic manager, replacing it with an empty one.
    ///
    /// # Returns
    ///
    /// The `DiagnosticManager` instance containing collected diagnostics.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        std::mem::take(&mut self.diagnostic_manager)
    }

    /// Returns an immutable reference to the diagnostic manager.
    ///
    /// Allows inspection of warnings, errors, or informational diagnostics
    /// collected during normalization.
    ///
    /// # Returns
    ///
    /// Reference to the internal `DiagnosticManager`.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the diagnostic manager.
    ///
    /// Allows adding or modifying diagnostics after normalization.
    ///
    /// # Returns
    ///
    /// Mutable reference to the internal `DiagnosticManager`.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }

    /// Returns a reference to the `StringInterner` associated with the AST if present,
    /// otherwise returns a reference to the local interner.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the `StringInterner`, or `None` if neither is available.
    pub fn interner(&self) -> Option<&StringInterner> {
        if let Some(ast) = &self.ast {
            Some(ast.interner())
        } else {
            self.interner.as_ref()
        }
    }

    /// Returns a mutable reference to the `StringInterner` associated with the AST if present,
    /// otherwise returns a mutable reference to the local interner.
    ///
    /// # Returns
    ///
    /// An `Option` containing a mutable reference to the `StringInterner`, or `None` if neither is available.
    pub fn interner_mut(&mut self) -> Option<&mut StringInterner> {
        if let Some(ast) = &mut self.ast {
            Some(ast.interner_mut())
        } else {
            self.interner.as_mut()
        }
    }

    /// Consumes and returns the `StringInterner` associated with the AST if present,
    /// otherwise consumes and returns the local interner.
    ///
    /// This leaves `None` in place of the interner in either location.
    ///
    /// # Returns
    ///
    /// An `Option<StringInterner>` containing the taken interner, or `None` if neither is present.
    pub fn take_interner(&mut self) -> Option<StringInterner> {
        if let Some(ast) = &mut self.ast {
            Some(std::mem::take(ast.interner_mut()))
        } else {
            self.interner.take()
        }
    }

    /// Returns `true` if parsing produced a valid AST.
    pub fn is_success(&self) -> bool {
        self.ast.is_some()
    }

    /// Returns `true` if parsing failed and no AST was produced.
    pub fn is_failure(&self) -> bool {
        self.ast.is_none()
    }
}

impl fmt::Display for NormalizerResult {
    /// Formats the normalization result for display.
    ///
    /// If an AST is present, it prints the root syntax of the AST.
    /// It then prints any diagnostics collected during normalization.
    ///
    /// If no AST is present, it notes that normalization failed.
    ///
    /// # Example output
    ///
    /// ```
    /// Normalized AST:
    /// (AST root syntax printed here)
    ///
    /// Normalization diagnostics:
    /// - Warning: ...
    /// - Error: ...
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.ast {
            Some(ast) => {
                writeln!(f, "Normalized AST:\n{}", ast.syntax_tree())?;
            }
            None => {
                writeln!(f, "No AST available (normalization failed).")?;
            }
        }

        if self.diagnostic_manager.is_empty() {
            writeln!(f, "\nNo issues during normalization.")
        } else {
            writeln!(f, "\nNormalization diagnostics:")?;
            for diag in self.diagnostic_manager.diagnostics() {
                writeln!(f, "- {}", diag)?;
            }
            Ok(())
        }
    }
}
