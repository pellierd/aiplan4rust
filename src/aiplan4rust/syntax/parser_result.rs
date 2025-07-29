//! Module defining the `ParserResult` type_checker, representing the outcome of a PDDL parsing operation.
//!
//! This structure combines the parsed abstract syntax tree (AST) and diagnostics produced during parsing,
//! enabling easy inspection of success or failure along with detailed error/warning information.

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::interner::StringInterner;

use std::fmt;

/// Represents the outcome of a PDDL syntax parsing operation.
///
/// `ParserResult` encapsulates two main components:
/// - An optional [`Ast`] containing the parsed abstract syntax tree if parsing succeeded.
/// - A [`DiagnosticManager`] holding diagnostics (errors, warnings, notes) generated during parsing.
///
/// This struct serves as a unified return type_checker for the parsing phase,
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
    interner: Option<StringInterner>,
}

impl ParserResult {
    /// Creates a successful `ParserResult` with a parsed AST.
    ///
    /// # Arguments
    /// - `ast`: The successfully parsed AST.
    /// - `diagnostic_manager`: The diagnostics collected during parsing.
    ///
    /// # Returns
    /// A `ParserResult` representing a successful parsing operation.
    pub fn success(ast: Ast, diagnostic_manager: DiagnosticManager) -> Self {
        Self {
            ast: Some(ast),
            diagnostic_manager,
            interner: None,
        }
    }

    /// Creates a failure `ParserResult` without a parsed AST.
    ///
    /// # Arguments
    /// - `diagnostic_manager`: The diagnostics collected during parsing.
    /// - `interner`: The interner used during parsing.
    ///
    /// # Returns
    /// A `ParserResult` representing a failed parsing operation.
    pub fn failure(diagnostic_manager: DiagnosticManager, interner: StringInterner) -> Self {
        Self {
            ast: None,
            diagnostic_manager,
            interner: Some(interner),
        }
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

    /// Returns a reference to the `StringInterner` associated with the AST
    /// or the local interner if the AST is absent.
    ///
    /// # Panics
    ///
    /// Panics if neither the AST nor the local interner is present.
    /// This should not happen if the invariant is respected.
    pub fn interner(&self) -> &StringInterner {
        if let Some(ast) = &self.ast {
            ast.interner()
        } else {
            // Assuming self.interner is always Some, else panic
            self.interner.as_ref()
                .expect("No interner available")
        }
    }

    /// Returns a mutable reference to the `StringInterner` associated with the AST
    /// or the local interner if the AST is absent.
    ///
    /// # Panics
    ///
    /// Panics if neither the AST nor the local interner is present.
    /// This should not happen if the invariant is respected.
    pub fn interner_mut(&mut self) -> &mut StringInterner {
        if let Some(ast) = &mut self.ast {
            ast.interner_mut()
        } else {
            self.interner.as_mut()
                .expect("No interner available")
        }
    }

    /// Consumes and returns the `StringInterner` associated with the AST
    /// or the local interner if the AST is absent.
    ///
    /// This leaves an empty `StringInterner` in place of the taken one.
    ///
    /// # Panics
    ///
    /// Panics if neither the AST nor the local interner is present.
    /// This should not happen if the invariant is respected.
    pub fn take_interner(&mut self) -> StringInterner {
        if let Some(ast) = &mut self.ast {
            std::mem::take(ast.interner_mut())
        } else {
            self.interner.take()
                .expect("No interner available")
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

impl fmt::Display for ParserResult {
    /// Formats the parser result as a human-readable string.
    ///
    /// If parsing succeeded, displays the AST followed by any diagnostics.
    /// If parsing failed, displays all diagnostics related to the failure,
    /// and, if available, the contents of the interner.
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

                if let Some(interner) = &self.interner {
                    write!(f, "\nInterner contents:\n")?;
                    // Suppose que StringInterner impl Display ou tu adaptes selon l’API de ton interner
                    write!(f, "{}", interner)?;
                }
            }
        }
        Ok(())
    }
}
