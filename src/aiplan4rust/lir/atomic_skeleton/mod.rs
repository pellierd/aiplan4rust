//! The `atomic_skeleton` module contains abstract skeletons common to predicates, tasks,
//! functions, methods, and actions in PDDL representation.
//!
//! This module defines base structures like `NamedTypedList` that encapsulate
//! shared parts such as the name and typed parameter list (signature).
//! These skeletons factorize common behavior and fields,
//! allowing uniform handling of atomic entities in the planner.
//!
//! # Main contents
//! - `NamedTypedList`: Abstract skeleton for named entities with a typed signature,
//!   used for predicates, functions, tasks, methods, and actions.
//! - `AtomicFormulaSkeleton`: Skeleton for atomic formulas (predicates).
//! - `AtomicFunctionSkeleton`: Skeleton for atomic functions.
//! - `AtomicTaskSkeleton`: Skeleton for atomic tasks.
//!
//! These skeletons simplification the definition and management of atomic elements
//! in PDDL domains and problems.
//!
//! # Example
//! ```rust
//! use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
//! use crate::aiplan4rust::lang::{Ident, TypedList};
//!
//! let named = NamedTypedList::new(Ident::new("move"), TypedList::new(vec![]));
//! println!("Entity name: {}", named.name());
//! ```

pub mod formula;
pub mod function;
pub mod task;
pub mod named_typed_list;

pub use formula::Formula as AtomicFormulaSkeleton;
pub use function::Function as AtomicFunctionSkeleton;
pub use task::Task as AtomicTaskSkeleton;
pub use named_typed_list::NamedTypedList;
