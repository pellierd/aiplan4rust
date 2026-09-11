//! Datalog Compilation and Logical Encoding Engine.
//!
//! This module serves as the core orchestration layer for transforming high-level PDDL
//! domain and problem representations into optimized, stack-allocated, and fully stratified
//! Datalog rules, facts, and constraints used for advanced reachability analysis.
//!
//! # Architectural Submodules
//!
//! * **[`action`]:** Compiles PDDL action definitions into Datalog rules, managing action signature
//!   naming, parameter type-anchoring, structural inertia analysis, and action bootstrap logic.
//! * **[`aliasing`]:** Provides zero-allocation, cache-localized equality unification and variable
//!   alias flattening via fixed-size flat arrays and bitmask cycle protection.
//! * **[`expr`]:** Translates complex logical expressions (nested `AND`/`OR` formulas, preconditions,
//!   and effects) into normalized Datalog Horn clauses with automatic safety enforcement.
//! * **[`facts`]:** Manages Datalog database initialization, type skeleton mapping, static object
//!   typing, and initial state AST ingestion.
//! * **[`predicate`] (private):** Handles auxiliary predicate signature generation and structural
//!   skeleton registration for intermediate logical subexpressions.
//!
//! # Performance & Design Philosophy
//!
//! The entire encoder pipeline adheres to a strict **zero-heap-allocation-first** philosophy on critical paths:
//! - Leveraging stack-allocated contiguous buffers (`SmallVec`) for term and clause construction.
//! - Utilizing CPU-register-sized bitmasks (`VariableMask`) to accelerate variable analysis and binding checks.
//! - Enforcing strict Datalog safety and stratification guarantees to ensure logical validity prior to execution.

pub(crate) mod action;
pub(crate) mod aliasing;
pub(crate) mod expr;
pub(crate) mod facts;
mod predicate;

pub(crate) use aliasing::AliasTable;
