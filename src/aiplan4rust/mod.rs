pub mod cli;
pub mod diagnostic;
pub mod file_format;
pub mod frontend;
pub mod linking;
pub mod syntax;
pub mod semantic;
pub mod normalization;
pub mod interner;
mod ir;

pub use file_format::FileFormat;
pub use frontend::Frontend;
pub use normalization::Normalizer;
pub use semantic::Analyzer;
pub use syntax::Parser;
pub use linking::Linker;
