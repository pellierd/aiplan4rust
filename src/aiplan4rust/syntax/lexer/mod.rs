/// This module exposes the core components for lexical analysis, including the lexer,
/// the token definitions, and lexical error handling.

/// The `lexer` module is responsible for tokenizing an input string.
/// It provides the `Lexer` struct for performing lexical analysis.
pub mod lexer;

/// The `lexical_error` module defines errors that can occur during the lexical analysis,
/// such as invalid tokens or number parsing errors.
pub mod lexical_error;

/// The `token` module defines the various token types that can be recognized in the input string,
/// such as numbers, operators, and keywords.
pub mod token;

/// Re-exporting the `Lexer` struct from the `lexer` module to make it accessible to other parts of
/// the code.
pub use lexer::Lexer;

/// Re-exporting the `Token` enum from the `token` module to make it accessible to other parts of
/// the code.
pub use token::Token;

/// Re-exporting the `LexicalError` enum from the `lexical_error` module to make it accessible to
/// other parts of the code.
pub use lexical_error::LexicalError;
