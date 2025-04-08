use crate::aiplan4rust::error::ErrorManager;
use crate::aiplan4rust::parser::syntax_tree::SyntaxTree;

use std::fmt;

/// A structure representing the result of a parsing operation.
///
/// The `ParserResult` contains an optional `SyntaxTree` and an `ErrorManager`.
/// The `SyntaxTree` is the output of the parsing process, while the `ErrorManager` tracks
/// errors that occurred during the parsing process, including recoverable errors.
///
/// This structure provides methods to access the `SyntaxTree` and manage the associated
/// errors through the `ErrorManager`.
///
/// # Fields
/// - `syntax_tree`: An `Option<SyntaxTree>` that may contain the parsed syntax tree, or `None`
///   if the parsing failed.
/// - `error_manager`: An instance of `ErrorManager` which handles the errors encountered
///   during parsing, including recoverable errors.
#[derive(Debug, Clone)]
pub struct ParserResult {
    syntax_tree: Option<SyntaxTree>,
    error_manager: ErrorManager,
}

impl ParserResult {
    /// Creates a new `ParserResult` with the given `SyntaxTree` and `ErrorManager`.
    ///
    /// # Arguments
    /// - `syntax_tree`: An optional `SyntaxTree` that represents the result of parsing.
    /// - `error_manager`: An `ErrorManager` that tracks errors encountered during parsing.
    ///
    /// # Returns
    /// A new `ParserResult` instance.
    pub fn new(syntax_tree: Option<SyntaxTree>, error_manager: ErrorManager) -> Self {
        ParserResult {
            syntax_tree,
            error_manager,
        }
    }

    /// Returns a reference to the `SyntaxTree` if available, or an error if no tree is present.
    ///
    /// # Returns
    /// - `Some(&SyntaxTree)`: A reference to the parsed syntax tree if parsing was successful.
    /// - `None`: If parsing failed and no syntax tree is available.
    pub fn syntax_tree(&self) -> Option<&SyntaxTree> {
        self.syntax_tree.as_ref()
    }

    /// Returns a mutable reference to the `SyntaxTree` if available, allowing modification of the
    /// tree.
    ///
    /// # Returns
    /// - `Some(&mut SyntaxTree)`: A mutable reference to the parsed syntax tree.
    /// - `None`: If no syntax tree is available, as the parsing has failed.
    pub fn syntax_tree_mut(&mut self) -> Option<&mut SyntaxTree> {
        self.syntax_tree.as_mut()
    }

    /// Returns a reference to the `ErrorManager`, which tracks errors encountered during parsing.
    ///
    /// # Returns
    /// A reference to the `ErrorManager` instance associated with this `ParserResult`.
    pub fn error_manager(&self) -> &ErrorManager {
        &self.error_manager
    }

    /// Returns a mutable reference to the `ErrorManager`, allowing modification of the error state.
    ///
    /// # Returns
    /// A mutable reference to the `ErrorManager` instance associated with this `ParserResult`.
    pub fn error_manager_mut(&mut self) -> &mut ErrorManager {
        &mut self.error_manager
    }

    /// Checks if the parsing was successful, i.e., the `SyntaxTree` is available.
    ///
    /// # Returns
    /// `true` if the parsing was successful (i.e., `syntax_tree` is `Some`), otherwise `false`.
    pub fn is_some(&self) -> bool {
        self.syntax_tree.is_some()
    }

    /// Checks if the parsing was unsuccessful, i.e., the `SyntaxTree` is not available.
    ///
    /// # Returns
    /// `true` if the parsing was unsuccessful (i.e., `syntax_tree` is `None`), otherwise `false`.
    pub fn is_none(&self) -> bool {
        self.syntax_tree.is_none()
    }
}
impl fmt::Display for ParserResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.syntax_tree {
            Some(tree) => {
                // If the syntax tree exists, display the tree and any errors.
                write!(f, "Parsing successful:\n{}", tree.root())?;
                if !self.error_manager.is_empty() {
                    write!(f, "\nErrors encountered during parsing:\n")?;
                    for error in self.error_manager.errors() {
                        write!(f, "{}\n", error)?;
                    }
                } else {
                    write!(f, "\nNo errors detected.")?;
                }
            }
            None => {
                // If no syntax tree is available, display parsing failure and errors.
                write!(f, "Parsing failed:\n")?;
                for error in self.error_manager.errors() {
                    write!(f, "{}\n", error)?;
                }
            }
        }
        Ok(())
    }
}
