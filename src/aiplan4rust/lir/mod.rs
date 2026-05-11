//! Lifted Intermediate Representation (LIR) module for AI Planning.
//!
//! This module defines the common components and abstractions for representing
//! AI syntax problems in a lifted (parameterized) intermediate representation.
//! It supports the construction, manipulation, and querying of lifted
//! syntax structures such as actions, methods, task networks, and problems.
//!
//! # Submodules
//!
//! - [`encoder`]: Provides the `LirBuilder` for constructing LIR entities
//!   using a builder pattern.
//! - [`expr`]: Contains definitions related to logic used within the LIR.
//! - [`problem`]: Defines the `LiftedProblem` struct representing a lifted syntax problem.
//! - [`action`]: Defines the `LiftedAction` struct for parameterized actions.
//! - [`method`]: Contains `LiftedMethod` representing hierarchical syntax methods.
//! - [`task_network`]: Defines `LiftedTaskNetwork` representing collections of tasks.
//! - [`initial_task_network`]: Represents the initial task network as input to planners.
//! - [`atomic_skeleton`]: Contains atomic or fundamental building blocks of LIR.
//! - [`result`]: Defines `LirBuilderResult`, the result type_checker for builder operations.
//! - [`error`]: Contains error types related to LIR construction and validation.
//!
//! # Public Exports
//!
//! - `LirBuilder`: Main builder for constructing lifted intermediate representations.
//! - `LiftedProblem`: The lifted problem abstraction.
//! - `LiftedAction`: Lifted action abstraction.
//! - `LiftedMethod`: Lifted method abstraction.
//! - `LiftedTaskNetwork`: Lifted task network abstraction.
//! - `InitialTaskNetwork`: Initial task network representation.
//! - `LirBuilderResult`: Result type_checker returned from builder operations.
//! - `LirError`: Error type_checker for LIR-related errors.
//!
//! # Example
//!
//! ```rust
//! use crate::aiplan4rust::lir::*;
//!
//! let builder = LirBuilder::new();
//! // Use builder to create lifted problems, actions, and methods.
//! ```
//!
//! This module is foundational for hierarchical and lifted AI syntax,
//! enabling complex task decompositions and parameterized syntax domains.

pub mod encoder;
pub mod expr;

pub mod encoding;
pub mod error;
mod passes;
pub mod problem;
pub mod renderers;
pub mod result;
pub(crate) mod store;

pub use encoder::LirEncoder;
pub use error::LirError;
pub use expr::Expr;
pub use problem::action::Action as ActionDef;
pub use problem::derived_predicate::DerivedPredicate as DerivedPredicateDef;
pub use problem::initial_task_network::InitialTaskNetwork;
pub use problem::method::Method as MethodDef;
pub use problem::task_network::TaskNetwork;
pub use result::Result as LirEncoderResult;
