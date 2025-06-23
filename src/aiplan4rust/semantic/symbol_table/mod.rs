pub mod table;

pub mod builder;
pub mod origin;

pub use table::Table as SymbolTable;
pub use origin::Origin as SymbolTableOrigin;

pub(crate) use builder::SymbolTableBuilder;
