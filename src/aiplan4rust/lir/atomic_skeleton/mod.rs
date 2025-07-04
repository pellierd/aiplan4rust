//! Atomic Skeleton Module
//!
//! This module defines the **atomic skeleton types** used to represent the signatures of
//! fundamental PDDL elements:
//!
//! - [`AtomicFormulaSkeleton`] : the signature of a predicate (name + parameters).
//! - [`AtomicFunctionSkeleton`] : the signature of a function (name + parameters + return type).
//! - [`AtomicTaskSkeleton`] : the signature of a planning task (name + parameters).
//!
//! These structures are commonly used in:
//! - The Low-level Intermediate Representation (LIR) of a planning domain.
//! - Semantic validation of declarations.
//! - Pretty-printing or serialization of domain components.
//!
//! # Re-exports
//!
//! This module re-exports the most common names under more explicit aliases:
//!
//! - `AtomicFormulaSkeleton` = [`Formula`]
//! - `AtomicFunctionSkeleton` = [`Function`]
//! - `AtomicTaskSkeleton` = [`Task`]
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::lir::atomic_skeleton::AtomicFunctionSkeleton;
//! use aiplan4rust::lang::{Ident, TypedList, Type};
//!
//! let func = AtomicFunctionSkeleton::new(
//!     Ident::new("distance"),
//!     TypedList::from(vec![]),
//!     Type::Number,
//! );
//!
//! assert_eq!(func.return_type(), &Type::Number);
//! ```
pub mod formula;
pub mod function;
pub mod task;

pub use formula::Formula as AtomicFormulaSkeleton;
pub use function::Function as AtomicFunctionSkeleton;
pub use task::Task as AtomicTaskSkeleton;
