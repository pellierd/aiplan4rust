//! Module defining the `ParserResult` type, representing the outcome of a PDDL parsing operation.
//!
//! This structure combines the parsed abstract syntax tree (AST) and diagnostics produced during parsing,
//! enabling easy inspection of success or failure along with detailed error/warning information.

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::syntax::ast::Ast;

use std::fmt;

/// Represents the outcome of a PDDL syntax parsing operation.
///
/// `ParserResult` encapsulates two main components:
/// - An optional [`Ast`] containing the parsed abstract syntax tree if parsing succeeded.
/// - A [`DiagnosticManager`] holding diagnostics (errors, warnings, notes) generated during parsing.
///
/// This struct serves as a unified return type for the parsing phase,
/// allowing straightforward success checks and detailed diagnostics inspection.
///
/// # Example
/// ```
/// let result = syntax.parse();
/// if result.is_some() {
///     println!("Parsed successfully!");
/// } else {
///     println!("Parsing failed: {:?}", result.diagnostic_manager().diagnostics());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ParserResult {
    ast: Option<Ast>,
    diagnostic_manager: DiagnosticManager,
}

impl ParserResult {
    /// Constructs a new `ParserResult`.
    ///
    /// # Parameters
    /// - `ast`: Optional parsed AST; `None` indicates parsing failure.
    /// - `diagnostic_manager`: Collection of diagnostics produced during parsing.
    ///
    /// # Returns
    /// A new `ParserResult` instance.
    pub fn new(ast: Option<Ast>, diagnostic_manager: DiagnosticManager) -> Self {
        Self { ast, diagnostic_manager }
    }

    /// Returns an immutable reference to the parsed AST, if available.
    ///
    /// # Returns
    /// - `Some(&Ast)` if parsing succeeded.
    /// - `None` if parsing failed.
    pub fn ast(&self) -> Option<&Ast> {
        self.ast.as_ref()
    }

    /// Returns a mutable reference to the parsed AST, if available.
    ///
    /// Allows modification of the AST after parsing.
    pub fn ast_mut(&mut self) -> Option<&mut Ast> {
        self.ast.as_mut()
    }

    /// Returns an immutable reference to the diagnostic manager.
    ///
    /// This contains all diagnostics (errors, warnings, notes) generated during parsing.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the diagnostic manager.
    ///
    /// Allows adding or modifying diagnostics after parsing.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }

    /// Takes ownership of the AST, leaving `None` in its place.
    pub fn take_ast(&mut self) -> Option<Ast> {
        self.ast.take()
    }

    /// Takes ownership of the diagnostic manager, replacing it with a default empty one.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        std::mem::take(&mut self.diagnostic_manager)
    }

    /// Returns `true` if parsing produced a valid AST.
    pub fn is_some(&self) -> bool {
        self.ast.is_some()
    }

    /// Returns `true` if parsing failed and no AST was produced.
    pub fn is_none(&self) -> bool {
        self.ast.is_none()
    }
}

impl fmt::Display for ParserResult {
    /// Formats the parser result as a human-readable string.
    ///
    /// If parsing succeeded, displays the AST followed by any diagnostics.
    /// If parsing failed, displays all diagnostics related to the failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.ast {
            Some(ast) => {
                write!(f, "Parsing successful:\n{}", ast)?;
                if !self.diagnostic_manager().is_empty() {
                    write!(f, "\nDiagnostics encountered during parsing:\n")?;
                    for diagnostic in self.diagnostic_manager().diagnostics() {
                        write!(f, "{}\n", diagnostic)?;
                    }
                } else {
                    write!(f, "\nNo diagnostics detected.")?;
                }
            }
            None => {
                write!(f, "Parsing failed:\n")?;
                for diagnostic in self.diagnostic_manager().diagnostics() {
                    write!(f, "{}\n", diagnostic)?;
                }
            }
        }
        Ok(())
    }
}
