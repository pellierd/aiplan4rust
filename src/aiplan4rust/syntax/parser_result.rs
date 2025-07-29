//! Module defining the `ParserResult` type_checker, representing the outcome of a PDDL parsing operation.
//!
//! This structure combines the parsed abstract syntax tree (AST) and diagnostics produced during parsing,
//! enabling easy inspection of success or failure along with detailed error/warning information.

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::syntax::ast::Ast;

use std::fmt;
use crate::aiplan4rust::interner::StringInterner;

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
    /// Constructs a new `ParserResult`.
    ///
    /// # Parameters
    /// - `ast`: An optional parsed AST. `None` indicates that parsing failed.
    /// - `diagnostic_manager`: The diagnostics collected during parsing.
    /// - `interner`: An optional `StringInterner` used during parsing.
    ///
    /// # Returns
    ///
    /// A new instance of `ParserResult`.
    pub fn new(
        ast: Option<Ast>,
        diagnostic_manager: DiagnosticManager,
        interner: Option<StringInterner>,
    ) -> Self {
        Self { ast, diagnostic_manager, interner }
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

    /// Takes ownership of the `StringInterner` associated with the AST if present,
    /// otherwise takes ownership of the local interner.
    ///
    /// This will remove the interner from either the AST or local storage,
    /// leaving `None` in its place if applicable.
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
