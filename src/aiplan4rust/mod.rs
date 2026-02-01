pub mod cli;
pub mod diagnostic;
pub mod linking;
pub mod syntax;
pub mod semantic;
pub mod normalization;
pub mod interner;
pub mod lir;
pub mod serialization;
pub mod lang;

pub mod frontend;
pub mod validation;
pub mod error;

pub mod artefact;
mod grounding;
pub mod arena;
pub mod tree;

pub use frontend::Frontend;
pub use normalization::Normalizer;
pub use semantic::Analyzer;
pub use syntax::Parser;
pub use linking::Linker;
pub use lir::LirBuilder;

pub use error::AiplanError;
