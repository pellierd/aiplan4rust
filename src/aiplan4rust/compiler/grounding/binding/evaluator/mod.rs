//! Unified Expression Evaluation and Constant Simplification Module.
//!
//! This module provides the support traits, error handling structures, and data representations
//! necessary to inject domain knowledge and static analysis markers directly into the Logical
//! Intermediate Representation (LIR) binding engine.
//!
//! By combining structural expression rewriting (`ExprBinder`) with static lookups via [`ExprEvaluator`],
//! the system can compress, inline, and prune entire evaluation trees during the grounding phase,
//! preventing redundant state allocations during subsequent planning routines.
//!
//! ### Error Handling Architecture
//!
//! To guarantee decoupled and extensible evaluation implementations, errors are managed via the
//! dynamic [`ExprEvaluatorError`] trait hierarchy. Concrete evaluation structures (such as inertia
//! or reachability engines) map their specific failures to this trait. A blanket `From` implementation
//! ensures frictionless type erasure into a standard `Box<dyn ExprEvaluatorError>`.

pub mod constant;
pub mod error;
pub mod evaluator;

pub use constant::ExprConstant;
pub use error::ExprEvaluatorError;
pub use evaluator::ExprEvaluator;
