use thiserror::Error;
use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Represents all possible errors that can occur while working with the AST
/// or its related parsing context.
///
/// This error type_checker encapsulates lower-level issues such as arena allocation failures,
/// syntax tree errors, as well as internal logic errors that may arise during AST
/// construction or transformation.
#[derive(Error, Debug)]
pub enum ParseContextError {
    /// An error occurred during arena allocation or manipulation.
    ///
    /// This usually indicates a failure in allocating or retrieving nodes
    /// from the AST arena data structure.
    #[error("Arena error: {0}")]
    Arena(#[from] ArenaError),

    /// An error related to the syntax tree structure or its operations.
    #[error("Syntax tree error: {0}")]
    SyntaxTree(#[from] SyntaxTreeError),

}
