use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::syntax::ast::{Ast, AstArena};

use std::fmt;

/// Represents the outcome of a parsing operation in the PDDL syntax.
///
/// This structure encapsulates two core elements:
/// - An optional [`Ast`] containing the parsed abstract syntax tree if parsing succeeded.
/// - A [`DiagnosticManager`] storing all diagnostics (errors, warnings, etc.) generated during parsing.
///
/// `ParserResult` serves as a unified return type for the parsing phase,
/// enabling straightforward inspection of success and detailed diagnostics access.
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
pub struct ArenaParserResult {
    ast: Option<AstArena>,
    diagnostic_manager: DiagnosticManager,
}

impl ArenaParserResult {
    /// Creates a new `ParserResult` from an optional AST and diagnostic manager.
    ///
    /// # Arguments
    ///
    /// * `ast_old` - The resulting AST from parsing, or `None` if parsing failed completely.
    /// * `diagnostic_manager` - Container for all diagnostics produced during parsing.
    pub fn new(ast: Option<AstArena>, diagnostic_manager: DiagnosticManager) -> Self {
        ArenaParserResult {
            ast,
            diagnostic_manager,
        }
    }

    /// Returns an immutable reference to the parsed AST if available.
    ///
    /// # Returns
    ///
    /// * `Some(&Ast)` if parsing succeeded.
    /// * `None` if parsing failed.
    pub fn ast(&self) -> Option<&AstArena> {
        self.ast.as_ref()
    }

    /// Returns a mutable reference to the parsed AST if available.
    ///
    /// Allows modifying the AST after parsing.
    pub fn ast_mut(&mut self) -> Option<&mut AstArena> {
        self.ast.as_mut()
    }

    /// Returns an immutable reference to the diagnostic manager.
    ///
    /// This contains all errors, warnings, and notes produced during parsing.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the diagnostic manager.
    ///
    /// Enables adding or modifying diagnostics post-parsing.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }

    /// Takes ownership of the AST, leaving `None` in its place.
    pub fn take_ast(&mut self) -> Option<AstArena> {
        self.ast.take()
    }

    /// Takes ownership of the diagnostic manager, replacing it with a default empty one.
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        std::mem::take(&mut self.diagnostic_manager)
    }

    /// Returns `true` if the parsing produced a valid AST.
    pub fn is_some(&self) -> bool {
        self.ast.is_some()
    }

    /// Returns `true` if parsing failed and no AST was produced.
    pub fn is_none(&self) -> bool {
        self.ast.is_none()
    }
}

impl fmt::Display for ArenaParserResult {
    /// Formats the parser result as a human-readable string.
    ///
    /// Displays the AST root if parsing succeeded, and lists all diagnostics.
    /// If parsing failed, displays all diagnostics related to the failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.ast {
            Some(tree) => {
                write!(f, "Parsing successful:\n{}", tree)?;
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
