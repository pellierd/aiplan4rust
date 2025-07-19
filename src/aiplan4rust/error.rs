use std::backtrace::Backtrace;
use std::fmt;

#[derive(Debug)]
pub struct AiplanError {
    pub message: String,
    pub backtrace: Backtrace,
}

impl AiplanError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            backtrace: Backtrace::capture(),
        }
    }
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }
}

impl fmt::Display for AiplanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ParserInternalError : {}", self.message)?;

        // Affiche la backtrace seulement en mode debug
        if cfg!(debug_assertions) {
            writeln!(f, "Backtrace :\n{}", self.backtrace)?;
        }

        Ok(())
    }
}
impl std::error::Error for AiplanError {}
