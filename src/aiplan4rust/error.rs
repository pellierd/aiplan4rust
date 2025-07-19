use std::backtrace::Backtrace;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AiplanError {
    Parser {
        message: String,
        backtrace: Backtrace,
    },
    // D’autres variantes à venir, ex Normalizer, Linker...
}

impl AiplanError {
    pub fn new(message: String) -> Self {
        AiplanError::Parser {
            message,
            backtrace: Backtrace::capture(),
        }
    }

    pub fn message(&self) -> &str {
        match self {
            AiplanError::Parser { message, .. } => message,
            // gérer les autres variantes plus tard
        }
    }

    pub fn backtrace(&self) -> &Backtrace {
        match self {
            AiplanError::Parser { backtrace, .. } => backtrace,
            // gérer les autres variantes plus tard
        }
    }
}

impl fmt::Display for AiplanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiplanError::Parser { message, backtrace } => {
                writeln!(f, "Parser error: {message}")?;

                if cfg!(debug_assertions) {
                    writeln!(f, "Backtrace:\n{backtrace}")?;
                }

                Ok(())
            }
        }
    }
}

impl Error for AiplanError {}
