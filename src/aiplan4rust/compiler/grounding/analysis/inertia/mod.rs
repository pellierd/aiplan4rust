//! Structural Inertia Analysis and Constant-Folding Engine.
//!
//! This module acts as the root boundary for analyzing, storing, and evaluating
//! planning invariants (structural rigidities) extracted from the initial state.
//! It provides the compiler's grounding phase with the necessary tools to prune
//! dead execution branches and fold constant functional terms early.
//!
//! # Module Architecture
//!
//! The inertia framework is decoupled into three core operational layers:
//!
//! * [`table`] — **Storage Layer**: Manages the memory layout, sparse multi-level matrices,
//!   and caching counting registers for both rigid predicates and functions.
//! * [`evaluator`] — **Execution Layer**: Implements the concrete hot-path tree traversal
//!   logic to short-circuit expression evaluations using the frozen tables.
//! * [`error`] — **Diagnostic Layer**: Centralizes and exposes [`InertiaError`], the unified
//!   error boundary that captures and encapsulates failures across all submodules.
//!
//! # Unified Error Boundary Routing
//!
//! To prevent internal implementation details from leaking into the main grounder loop,
//! this root module serves as an error unification frontier. Sub-errors such as
//! `InertiaTableError` and `InertiaEvaluatorError` are captured and automatically transformed
//! into the top-level [`InertiaError`] enum, simplifying upstream error handling.

pub mod error;
pub mod evaluator;
pub mod inertia;
pub mod table;

pub use error::InertiaError;
