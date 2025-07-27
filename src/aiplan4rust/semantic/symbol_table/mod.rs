pub mod table;

pub mod builder;
pub mod origin;
pub mod error;

pub use table::Table as SymbolTable;
pub use error::SymbolTableError;
pub use origin::Origin as SymbolTableOrigin;

pub(crate) use builder::SymbolTableBuilder;
