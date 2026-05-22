pub mod simplification;
pub mod rewriting;
pub mod error;

pub mod evaluator;
mod normalization;

pub use simplification::simplify;
pub use simplification::simplify_with;
pub use simplification::simplify_subexpr;
pub use simplification::simplify_subexpr_with;

pub use normalization::normalize;
pub use evaluator::StaticEvaluator;
pub use evaluator::StaticValue;
pub use error::ExprOpError;
