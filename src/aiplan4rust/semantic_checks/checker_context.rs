// checker_context.rs

use crate::aiplan4rust::diagnostic::DiagnosticSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckerContext {
    Parser,
    SemanticAnalyzer,
    Linker,
}

impl From<CheckerContext> for DiagnosticSource {
    fn from(ctx: CheckerContext) -> Self {
        match ctx {
            CheckerContext::Parser => DiagnosticSource::Parser,
            CheckerContext::SemanticAnalyzer => DiagnosticSource::SemanticAnalyzer,
            CheckerContext::Linker => DiagnosticSource::Linker,
        }
    }
}
