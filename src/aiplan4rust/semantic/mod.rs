pub mod analyzer;
pub mod symbol;
pub mod analyzer_result;
pub mod symbol_table;
pub mod checks;
pub mod context;
pub mod error;
mod type_checker;

pub use analyzer_result::AnalyzerResult;
pub use analyzer::Analyzer;
pub use symbol_table::SymbolTable;
pub use type_checker::type_checker::TypeChecker;
pub use error::SemanticError;
pub use context::Context as SemanticContext;
