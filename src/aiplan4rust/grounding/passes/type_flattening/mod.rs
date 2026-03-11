//! # Type Flattening Pass
//!
//! This module implements the transformation pass responsible for "flattening" 
//! the type hierarchy within a planning problem.
//!
//! ## Architecture
//! The module follows a **Facade** design pattern:
//! * **Orchestration**: The [`flatten`] function serves as the main entry point.
//! * **Internal State**: The [`PivotTracker`] manages type substitutions internally 
//!   to ensure consistency across all sub-modules.
//! * **Encapsulation**: All internal rewriting logic and the tracker are kept 
//!   private to this crate, exposing only the high-level transformation.

// --- Main Orchestration ---
mod problem;

// --- Rewriting Components ---
mod expr;
mod typed_symbol;
mod typed_list;
mod ty;
mod atomic_formula_skeleton;
mod atomic_function_skeleton;
mod derived_predicate;
mod task;
mod action;
mod method;
mod initial_task_network;

// --- Utilities ---
mod pivot_tracker;

// --- Testing ---
#[cfg(test)]
mod problem_tests;

// --- Public API ---
pub use problem::flatten;

// --- Internal Exports ---
// Available throughout the crate but hidden from the public API.
pub(crate) use pivot_tracker::PivotTracker;
