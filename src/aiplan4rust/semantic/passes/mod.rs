mod derived_resolution;
mod error;
mod symbol_resolution;

pub use derived_resolution::resolve_derived_predicates;
pub use error::SemanticPassError;
pub use symbol_resolution::resolve_symbols;
