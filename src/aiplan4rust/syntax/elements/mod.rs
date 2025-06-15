//! Operators and requirements module for the `aiplan4rust` crate.
//!
//! This module defines core enumerations and types related to PDDL/HDDL operators,
//! assignment operations, binary comparisons, optimization criteria, and
//! requirements declarations used in planning domain definitions.
//!
//! # Submodules
//!
//! - `arithmetic_op`: Defines arithmetic operations such as addition, subtraction, etc.
//! - `assign_op`: Defines assignment operations like assign, increase, decrease, etc.
//! - `binary_comp`: Defines binary comparison operators like `<`, `<=`, `=`, etc.
//! - `optimization`: Represents optimization criteria in planning (e.g., minimize, maximize).
//! - `requirement`: Enumerates domain requirements that specify language features used.
//!
//! # Re-exports
//!
//! The following types are publicly re-exported for convenience:
//! - [`ArithmeticOp`]
//! - [`AssignOp`]
//! - [`BinaryComp`]
//! - [`Optimization`]
//! - [`Requirement`]
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::syntax::elements::{ArithmeticOp, AssignOp, BinaryComp, Optimization, Requirement};
//!
//! // Example: creating an arithmetic operation
//! let op = ArithmeticOp::Add;
//! ```

pub mod arithmetic_op;
pub mod assign_op;
pub mod binary_comp;
pub mod optimization;
pub mod requirement;

pub use arithmetic_op::ArithmeticOp;
pub use assign_op::AssignOp;
pub use binary_comp::BinaryComp;
pub use optimization::Optimization;
pub use requirement::Requirement;
