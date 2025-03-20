use logos::{Logos, SpannedIter};

use crate::aiplan4rust::syntax::token::{LexicalError, Token};

/// Type alias for a `Result` that includes token location information (start and end positions).
///
/// This type represents a spanned token, which includes the location in the input string,
/// the token itself, and any possible errors associated with lexing. The location is represented
/// by the start and end positions in the input string.
///
/// The `Error` type can be a `LexicalError`, which indicates issues in lexing (e.g., invalid
/// tokens).
pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

/// `Lexer` struct that iterates over the input string and produces tokens.
pub struct Lexer<'input> {
    /// A token stream iterator that produces spanned tokens from the input.
    /// The token stream is an iterator that yields `Token` variants, along with
    /// their associated start and end positions in the input string.
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
    ///
    /// The method uses the `Token::lexer()` method from the `Logos` crate to tokenize
    /// the input string and return an iterator over the token stream.
    pub fn new(input: &'input str) -> Self {
        // Using the `Token::lexer(input).spanned()` to create the token stream.
        Self {
            token_stream: Token::lexer(input).spanned(),
        }
    }
}

/// Implements the `Iterator` trait for `Lexer`.
///
/// The `Lexer` struct implements the `Iterator` trait to produce tokens during iteration.
/// It yields tokens one by one, each with its associated start and end positions in the input
/// string. If an invalid token is encountered, it is returned as an error.
impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Token, usize, LexicalError>;

    /// Returns the next token from the token stream.
    ///
    /// The method processes the token stream to return a token along with its position in the
    /// input string. If an error occurs while lexing, an error token is returned instead.
    ///
    /// # Returns
    ///
    /// `Some(Ok(...))` if a valid token is found, or `Some(Err(...))` if a lexing error occurs.
    /// `None` is returned when the end of the input string is reached.
    fn next(&mut self) -> Option<Self::Item> {
        match self.token_stream.next() {
            None => {
                // End of the token stream; no more tokens to process
                None
            }
            Some((token, span)) => {
                match token {
                    // Token is an error (invalid token encountered)
                    Err(_) => {
                        let c = self.token_stream.slice().to_string();
                        // Returning the invalid token wrapped in a Token::Error variant
                        let t = Token::Error(c);
                        /*println!(
                            "ERROR Token: \'{}\' start: {} end: {}",
                            t.symbol(),
                            span.start,
                            span.end
                        );*/
                        // Returning the spanned error token with its start and end positions
                        Some(Ok((span.start, t, span.end)))
                    }
                    // Token is valid; wrap it in the result and return
                    Ok(t) => {
                        /*println!(
                            "OK Token: \'{}\' start: {} end: {}",
                            t.symbol(),
                            span.start,
                            span.end
                        );*/
                        Some(Ok((span.start, t, span.end)))
                    }
                }
            }
        }
    }
}
