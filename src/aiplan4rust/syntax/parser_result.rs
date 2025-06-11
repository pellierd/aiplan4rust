use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::syntax::ast::Ast;
use std::fmt;

/// Represents the outcome of a parsing operation in the PDDL syntax.
///
/// This structure encapsulates two core elements:
/// - An optional [`AstNode`], which is produced only if parsing succeeds without unrecoverable errors.
/// - A [`DiagnosticManager`] that stores all diagnostics (errors, warnings, etc.) generated during parsing.
///
/// The `ParserResult` acts as a unified return type for the parsing phase,
/// making it easier to inspect whether parsing succeeded, and to access detailed error reports.
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
    syntax_tree: Option<Ast>,
    diagnostic_manager: DiagnosticManager,
}

impl ParserResult {
    /// Constructs a new `ParserResult` with a given syntax tree and diagnostic manager.
    ///
    /// # Arguments
    /// * `ast` - The resulting syntax tree, or `None` if parsing failed completely.
    /// * `diagnostic_manager` - A manager that tracks diagnostics emitted during parsing.
    pub fn new(syntax_tree: Option<Ast>, diagnostic_manager: DiagnosticManager) -> Self {
        ParserResult {
            syntax_tree,
            diagnostic_manager,
        }
    }

    /// Returns an immutable reference to the parsed syntax tree, if available.
    ///
    /// # Returns
    /// * `Some(&SyntaxTree)` if parsing succeeded.
    /// * `None` if parsing failed.
    pub fn syntax_tree(&self) -> Option<&Ast> {
        self.syntax_tree.as_ref()
    }

    /// Returns a mutable reference to the parsed syntax tree, if available.
    ///
    /// This allows further modifications to the tree after parsing.
    pub fn syntax_tree_mut(&mut self) -> Option<&mut Ast> {
        self.syntax_tree.as_mut()
    }


    /// Returns an immutable reference to the diagnostic manager.
    ///
    /// The diagnostic manager contains all diagnostics produced during the parsing process.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Returns a mutable reference to the diagnostic manager.
    ///
    /// Allows appending new diagnostics or modifying the internal state.
    pub fn diagnostic_manager_mut(&mut self) -> &mut DiagnosticManager {
        &mut self.diagnostic_manager
    }

    // Prend la possession de l'AST
    pub fn take_ast(&mut self) -> Option<Ast> {
        self.syntax_tree.take()
    }

    // Prend la possession des diagnostics
    pub fn take_diagnostic_manager(&mut self) -> DiagnosticManager {
        std::mem::take(&mut self.diagnostic_manager)
    }

    /// Returns `true` if parsing succeeded and a syntax tree is available.
    pub fn is_some(&self) -> bool {
        self.syntax_tree.is_some()
    }

    /// Returns `true` if parsing failed and no syntax tree was produced.
    pub fn is_none(&self) -> bool {
        self.syntax_tree.is_none()
    }
}

impl fmt::Display for ParserResult {
    /// Formats the syntax result into a human-readable string.
    ///
    /// Displays whether the parsing was successful, the root of the syntax tree (if any),
    /// and all associated diagnostics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.syntax_tree {
            Some(tree) => {
                // If the syntax tree exists, display the tree and any errors.
                write!(f, "Parsing successful:\n{}", tree.root())?;
                if !self.diagnostic_manager().is_empty() {
                    write!(f, "\nErrors encountered during parsing:\n")?;
                    for diagnostic in self.diagnostic_manager().diagnostics() {
                        write!(f, "{}\n", diagnostic)?;
                    }
                } else {
                    write!(f, "\nNo errors detected.")?;
                }
            }
            None => {
                // If no syntax tree is available, display parsing failure and errors.
                write!(f, "Parsing failed:\n")?;
                for diagnostic in self.diagnostic_manager().diagnostics() {
                    write!(f, "{}\n", diagnostic)?;
                }
            }
        }
        Ok(())
    }
}
