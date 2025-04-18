use logos::Logos;
use logos::SpannedIter;
use log::debug;

use crate::aiplan4rust::parser::lexer::LexicalError;
use crate::aiplan4rust::parser::lexer::Token;

/// Type alias for a `Result` that includes token location information (start and end positions).
///
/// This type represents a spanned token, which includes the location in the input string,
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
    type Item = Spanned<Token, usize, LexicalError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.token_stream.next() {
            None => {
                debug!("Lexer reached EOF.");
                None
            }
            Some((token, span)) => match token {
                Err(_) => {
                    let c = self.token_stream.slice().to_string();
                    let t = Token::Error(c);
                    debug!(
                        "ERROR Token: '{}' at [{}..{}]",
                        t.symbol(),
                        span.start,
                        span.end
                    );
                    Some(Ok((span.start, t, span.end)))
                }
                Ok(t) => {
                    debug!(
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
