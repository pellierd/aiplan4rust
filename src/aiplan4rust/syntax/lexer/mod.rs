//! Lexical analysis module for the `aiplan4rust` crate.
//!
//! This module provides the tree components for lexical analysis of PDDL/HDDL source files,
//! including tokenization, token definitions, and lexical error handling.
//!
//! # Submodules
//!
//! - `lexer`: Contains the `Lexer` struct responsible for converting raw input strings into tokens.
//! - `lexical_error`: Defines errors that can occur during tokenization, such as invalid tokens or
//!   malformed numbers.
//! - `token`: Defines the various token types recognized by the lexer, including keywords, symbols,
//!   and literals.
//!
//! # Re-exports
//!
//! For ease of use, the following types are re-exported:
//! - [`Lexer`]
//! - [`Token`]
//! - [`LexicalError`]
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::syntax:lexer::{Lexer, Token, LexicalError};
//!
//! let source = "(define (domain test-domain))";
//! let mut lexer = Lexer::new(source);
//! while let Some(token_result) = lexer.next() {
//!     match token_result {
//!         Ok(token) => println!("Token: {:?}", token),
//!         Err(e) => eprintln!("Lexical error: {:?}", e),
//!     }
//! }
//! ```

pub mod lexer;
pub mod lexical_error;
pub mod token;

pub use lexer::Lexer;
pub use token::Token;
pub use lexical_error::LexicalError;
