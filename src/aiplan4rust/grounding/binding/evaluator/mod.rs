//! Unified Expression Evaluation and Constant Simplification Module.
//!
//! This module provides the core traits and data structures necessary to inject domain knowledge
//! and static analysis markers directly into the Logical Intermediate Representation (LIR) binding engine.
//!
//! By combining structural expression rewriting (`ExprBinder`) with static lookups via [`ExprEvaluator`],
//! the system can compress, inline, and prune entire evaluation trees during the grounding phase,
//! preventing redundant state allocations during subsequent planning routines.

pub mod constant;
pub mod evaluator;

#[doc(inline)]
pub use constant::ExprConstant;

#[doc(inline)]
pub use evaluator::ExprEvaluator;
