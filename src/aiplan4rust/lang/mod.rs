//! Fundamental language components used across syntax and semantics.
//!
//! This module defines essential building blocks that are shared between
//! multiple stages of the syntax language infrastructure, including:
//! - **Parsing and syntax trees (AST)**,
//! - **Semantic representation (IR)**,
//! - **Validation, transformation, and optimization**.
//!
//! These types serve as the common vocabulary for representing identifiers,
//! typed entities, operators, and logical/optimization constructs in the language.
//!
//! # Re-exported Modules and Types
//!
//! - [`Type`]: Represents a type_checker or a union of types.
//! - [`TypedSymbol`]: A named symbol with one or more associated types.
//! - [`TypedList`]: A list of typed symbols (commonly used for parameters, variables, etc.).
//! - [`Requirement`]: Declared requirements that affect parsing and validation.
//! - [`ArithmeticOp`]: Arithmetic operations (`+`, `-`, `*`, `/`) used in logic.
//! - [`AssignOp`]: Assignment-style operations for modifying fluent values.
//! - [`CompareOp`]: Binary comparison operators used in conditions and constraints.
//! - [`OptimizationOp`]: Declares whether to minimize, maximize, or ignore optimization goals.
//!
//! # Usage Scope
//!
//! These components are:
//! - Used in the **AST layer** to represent parsed syntax elements,
//! - Carried over into the **IR layer** for further analysis or execution,
//! - Shared across **formatters**, **displays**, and **serializers**.
//!
//! # Design Notes
//!
//! Most of these types are designed to be:
//! - **Serializable** (via `serde`) for I/O or debugging,
//! - **Interned** when applicable, to reduce memory duplication,
//! - **Composable** and **lightweight**, allowing reuse across modules without tight coupling.
//!
//! This centralization improves consistency and makes the language model easier to evolve.

pub mod ty;
pub mod typed_symbol;
pub mod typed_list;
pub mod requirement;
pub mod arithmetic_op;
pub mod assign_op;
pub mod compare_op;
pub mod optimization_op;
pub mod error;
pub mod ids;

pub use ty::Type;
pub use typed_symbol::TypedSymbol;
pub use typed_list::TypedList;
pub use requirement::Requirement;
pub use arithmetic_op::ArithmeticOp;
pub use assign_op::AssignOp;
pub use compare_op::CompareOp;
pub use optimization_op::OptimizationOp;
pub use error::LangError;
pub use crate::aiplan4rust::interner::remap_symbol::RemapSymbol;
pub use ids::*;
