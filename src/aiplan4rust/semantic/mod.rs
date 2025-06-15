pub mod analyzer;
pub mod symbol;
pub mod analyzer_result;
pub mod type_checker;
pub mod symbol_table;
pub mod checks;
pub mod hir;

pub use analyzer_result::AnalyzerResult;
pub use analyzer::Analyzer;
pub use symbol_table::SymbolTable;
pub use type_checker::TypeChecker;
pub use symbol_table::SymbolTableBuilder;
