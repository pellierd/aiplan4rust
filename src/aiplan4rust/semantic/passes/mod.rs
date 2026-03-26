pub mod ast_finalization;
pub mod context;
pub mod error;
pub mod type_simplification;

pub use ast_finalization::finalize;
pub use type_simplification::simplify_symbol_table;

pub use context::PassContext;
