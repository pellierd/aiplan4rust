pub mod simplify;
pub mod rewrite;
pub mod engine;
pub mod error;

pub mod evaluator;

pub use engine::LogicEngine;
pub use error::LogicError;


pub use evaluator::{StaticEvaluator, StaticValue};
