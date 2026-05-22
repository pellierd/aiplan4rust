//! # Type Normalization Pass
//!
//! This module implements the transformation pass responsible for "normalizing"
//! the type system within a Lifted Intermediate Representation (LIR).
//!
//! ## Overview
//! The primary goal of this pass is to eliminate composite types (specifically `either`
//! types) and missing root types from the LIR. It resolves these unions into unified
//! atomic identifiers, ensuring that the grounding engine and downstream solvers
//! operate on a simplified, non-hierarchical type space.
//!
//! ## Architecture
//! The module follows a **Facade** design pattern to manage the complexity of
//! full-problem transformation:
//!
//! * **Orchestration**: The [`normalize`] function serves as the main entry point,
//!     coordinating the visit across all problem components (actions, tasks, etc.).
//! * **Centralized Registry**: The [`TypeRegistry`] acts as the single source of
//!     truth for type unification, ensuring that a specific set of members always
//!     resolves to the same [`TypeId`].
//! * **Specialized Visitors**: Sub-modules handle the localized logic for
//!     transforming specific structures (e.g., expression trees, atomic formulae,
//!     and HTN task networks).
//!
//! ## Encapsulation
//! To maintain a clean public API, all internal simplification logic and the registry
//! are kept private to the `typing` module. Only the high-level [`normalize`]
//! function is exposed to the rest of the compiler crate.

// --- Main Orchestration ---
mod problem;

// --- Rewriting Components ---
mod action;
mod atomic_formula_skeleton;
mod atomic_function_skeleton;
mod derived_predicate;
mod expr;
mod initial_task_network;
mod method;
mod task;
mod ty;
mod typed_list;
mod typed_symbol;

// --- Utilities ---
mod registry;

// --- Public API ---
pub use problem::normalize;

// --- Internal Exports ---
// Internal helpers available to sub-modules within this pass.
pub(crate) use registry::TypeRegistry;
