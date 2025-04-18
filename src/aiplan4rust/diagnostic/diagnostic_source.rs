use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum DiagnosticSource {
    Lexer,
    Parser,
    SemanticAnalyzer,
    Linker,
}

impl fmt::Display for DiagnosticSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source_str = match self {
            DiagnosticSource::Lexer => "Lexer",
            DiagnosticSource::Parser => "Parser",
            DiagnosticSource::SemanticAnalyzer => "Semantic Analyzer",
            DiagnosticSource::Linker => "Linker",
        };
        write!(f, "{}", source_str)
    }
}
