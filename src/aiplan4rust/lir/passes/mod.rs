//! # LIR Transformation Passes
//!
//! This module manages the post-encoding pipeline for the Lifted Intermediate Representation (LIR).
//! It organizes various transformation "finalization" that refine the raw encoded problem into a
//! standardized format suitable for the grounding engine.
//!
//! ## Module Architecture
//!
//! The transformation is divided into two specialized sub-modules:
//!
//! * **[`logic`]**: Focuses on **Semantic Normalization**. It rewrites the logic of expressions
//!   (e.g., simplifying boolean operators, removing implications) without altering the
//!   problem's type structure.
//! * **[`typing`]**: Focuses on **Structural Normalization**. It handles "Type Flattening"
//!   by resolving `either` types into atomic types and updating the global symbol registry.
//!
//! ## The Facade Pattern
//!
//! To maintain a clean API, the internal complexity of these finalization is hidden. The
//! [`normalization`] module orchestrates the execution order, and the top-level
//! [`normalize`] function is re-exported as the single point of contact for the encoder.

mod logic;
mod typing;

/// Orchestration logic for running multiple finalization in the correct sequence.
pub mod normalization;

// Re-export the primary entry point for convenience.
// This allows callers to use `finalization::normalize(&mut problem)` directly.
pub use normalization::normalize;
