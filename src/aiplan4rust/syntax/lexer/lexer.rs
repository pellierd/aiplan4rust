//! This module defines a `Lexer` struct that tokenizes an input string using the `logos` crate,
//! producing a stream of spanned tokens with lexical error handling.
//!
//! The `Spanned` type_checker alias represents a `Result` containing a tuple with start and end locations
//! alongside a token or a lexical error. This allows tracking the precise position of each token
//! in the input text.
//!
//! The `Lexer` struct wraps a `logos::SpannedIter` over the `Token` enum, iterating over tokens
//! and producing them with their span (start and end offsets).
//!
//! The `Iterator` implementation for `Lexer` handles token extraction:
//! - Successfully parsed tokens are returned with their span.
//! - Tokens representing lexing errors are wrapped in an `Error` token variant along with the problematic slice.
//! - Debug logs are emitted for both normal tokens and errors, reporting token symbols and their spans.
//!
//! This lexer provides a foundation for lexical analysis with detailed error reporting and
//! position tracking, useful in parsing and compiling source code.

use logos::Logos;
use logos::SpannedIter;
use log::trace;

use crate::aiplan4rust::syntax::CustomParseError;
use crate::aiplan4rust::syntax::lexer::Token;

/// Type alias for a `Result` that includes token location information (start and end positions).
///
/// This type_checker represents a spanned token, which includes the location in the input string,
/// the token itself, and any possible errors associated with lexing. The location is represented
/// by the start and end positions in the input string.
pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

/// `Lexer` struct that iterates over the input string and produces tokens.
pub struct Lexer<'input> {
    /// A token stream iterator that produces spanned tokens from the input.
    token_stream: SpannedIter<'input, Token>,
}

impl<'input> Lexer<'input> {
    /// Creates a new `Lexer` from an input string.
    ///
    /// # Arguments
    ///
    /// * `input` - The input string that will be lexically analyzed by the lexer.
    ///
    /// # Returns
    ///
    /// A new `Lexer` instance containing a spanned token stream.
    pub fn new(input: &'input str) -> Self {
        Self {
            token_stream: Token::lexer(input).spanned(),
        }
    }
}

/// Implements the `Iterator` trait for `Lexer`.
impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Token, usize, CustomParseError>;

    /// Advances the lexer and returns the next token with its span.
    ///
    /// This method attempts to fetch the next token from the internal token stream.
    /// - If the stream is exhausted, it returns `None`.
    /// - If the lexer encounters a lexical error, it returns a `Token::Error`
    ///   wrapping the erroneous slice as a string, along with its span.
    /// - Otherwise, it returns the successfully parsed token with its span.
    ///
    /// # Returns
    ///
    /// - `Some(Ok((start, token, end)))` on successful tokenization,
    ///   where `start` and `end` mark the byte offsets of the token in the input.
    /// - `Some(Err(LexicalError))` if a lexical error occurred (wrapped inside `Token::Error` here).
    /// - `None` when the end of input is reached.
    ///
    /// # Debug Logging
    ///
    /// Logs debug messages indicating when tokens or errors are emitted,
    /// including the token’s symbol and span.
    fn next(&mut self) -> Option<Self::Item> {
        match self.token_stream.next() {
            None => {
                trace!("Lexer reached EOF.");
                None
            }
            Some((token, span)) => match token {
                Err(_) => {
                    let c = self.token_stream.slice().to_string();
                    let t = Token::Error(c);
                    trace!(
                        "ERROR Token: '{}' at [{}..{}]",
                        t.symbol(),
                        span.start,
                        span.end
                    );
                    Some(Ok((span.start, t, span.end)))
                }
                Ok(t) => {
                    trace!(
                        "OK Token: '{}' at [{}..{}]",
                        t.symbol(),
                        span.start,
                        span.end
                    );
                    Some(Ok((span.start, t, span.end)))
                }
            },
        }
    }
}
