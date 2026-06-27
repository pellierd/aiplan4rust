/// Quantifier Normal Form (QNF) processing pass module.
///
/// This module provides the core pipeline for expanding logical quantifiers (`forall` and `exists`)
/// into explicit grounded expressions (such as conjunctions and disjunctions) across all structural
/// components of a planning problem (actions, methods, derived predicates, constraints, and goals).
///
/// To eliminate hot-path allocations during tree traversal, it utilizes a pre-allocated
/// [`QnfScratchpad`] which is shared across the distinct sub-modules.
mod action;
mod derived_predicate;
mod expr;
mod method;
pub mod problem;
mod scratchpad;

pub(crate) use scratchpad::QnfScratchpad;
