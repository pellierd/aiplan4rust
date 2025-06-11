use std::fmt;

#[derive(Clone, Debug, Copy, PartialEq)]
pub enum Provider {
    Lexer,
    Parser,
    Analyzer,
    Linker,
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source_str = match self {
            Provider::Lexer => "Lexer",
            Provider::Parser => "Parser",
            Provider::Analyzer => "Analyzer",
            Provider::Linker => "Linker",
        };
        write!(f, "{}", source_str)
    }
}
