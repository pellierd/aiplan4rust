pub mod cli;
pub mod diagnostic;
pub mod linking;
pub mod syntax;
pub mod semantic;
pub mod normalization;
pub mod interner;
mod lir;
mod tree;
pub mod serialization;
pub mod lang;

pub mod frontend;

pub use frontend::Frontend;
pub use normalization::Normalizer;
pub use semantic::Analyzer;
pub use syntax::Parser;
pub use linking::Linker;
pub use lir::LIRBuilder;
