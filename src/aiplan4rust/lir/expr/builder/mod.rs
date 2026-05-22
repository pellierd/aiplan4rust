//! Expression Construction System
//!
//! This module provides a comprehensive API for building Planning Intermediate
//! Representation (LIR) expressions. It is structured around the [`ExprBuilder`]
//! which coordinates various sub-modules to handle different domains of the
//! Planning Domain Definition Language (PDDL) and HTN.
//!
//! # Architecture
//!
//! The builder is divided into several specialized sub-modules, each extending
//! [`ExprBuilder`] with specific "smart constructors":
//!
//! * **Logic & Comparison**: [`logical`], [`comparison`], [`atoms`].
//! * **Arithmetic & Metrics**: [`arithmetic`], [`assignment`], [`metric`].
//! * **Temporal & Constraints**: [`time`], [`constraints`], [`preferences`].
//! * **Hierarchical & Structural**: [`tasks`], [`symbols`], [`quantifiers`].
//!
//! # Smart Construction
//!
//! Unlike direct interning, the methods provided by this system perform:
//! 1.  **Deduplication**: Automatically reuses existing nodes in the [`ExprStore`].
//! 2.  **Canonicalization**: Sorts operands (e.g., in `AND` or `SUM`) to ensure
//!     commutative operations result in the same ID.
//! 3.  **Semantic Validation**: Checks for PDDL violations (e.g., nested temporal
//!     operators) via [`ExprBuilderError`].

mod arithmetic;
mod assignment;
mod atoms;
mod builder;
mod comparison;
mod constraints;
mod error;
mod logical;
mod metric;
mod preferences;
mod quantifiers;
mod symbols;
mod tasks;
mod time;

// Re-exports
pub use builder::ExprBuilder;
pub use error::ExprBuilderError;
