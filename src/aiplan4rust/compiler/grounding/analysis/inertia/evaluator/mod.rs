//! # Inertia Evaluator Module (Inertia Pattern Pruning - IPP)
//!
//! This module implements the static evaluation and simplification engine based on
//! Inertia Analysis, adhering to the principles outlined in the IPP planning framework
//! (Koehler et al., Definition 5 & 6).
//!
//! ## Core Responsibility
//! During the grounding (instantiation) phase of the PDDL compiler, this engine pre-evaluates
//! predicates and functions detected as **static** (exhibiting either strict Positive or Negative
//! Inertia). Because their truth values or numeric outputs cannot be modified by any action
//! schema within the planning domain, their state can be evaluated at compile-time,
//! drastically reducing the search space and pruning unreachable action branches.
//!
//! ## Submodule Architecture
//! To maintain scalability, the internal implementation of the [`InertiaEvaluator`]
//! is decoupled across specialized, non-public submodules:
//!
//! * [`evaluator`]: Houses the main [`InertiaEvaluator`] structural definitions and its builder pattern API.
//! * [`init`]: Manages initial state processing, compiling facts, and generating combinatorial Big-Endian bitmasks.
//! * [`predicate`]: Enforces atomic predicate pruning and logic pruning criteria ($N$ vs $\text{MAX}$ instance counting).
//! * [`function`]: Evaluates numeric fluents and implements standard PDDL fallback behaviors (e.g., default `0.0` values).
//! * [`error`]: Consolidates the error framework via [`InertiaEvaluatorError`].

pub mod error;
pub mod evaluator;
mod function;
mod init;
mod predicate;

pub use error::InertiaEvaluatorError;
pub use evaluator::InertiaEvaluator;
