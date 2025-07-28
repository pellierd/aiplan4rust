//! Module providing utilities for working with ordered floating-point numbers,
//! serialization helpers, syntax definitions, and related error types.
//!
//! This module re-exports key components:
//! - `serialize_ordered_float` and `deserialize_ordered_float` for (de)serializing
//!   `OrderedFloat<f64>` values.
//! - The `SerializationError` enum for handling serialization-related errors.
//!
//! Submodules:
//! - `serde`: Contains (de)serialization implementations for ordered floats.
//! - `syntax`: Defines syntax-related types and utilities.
//! - `error`: Defines error types related to serialization processes.

pub mod ordered_float;
pub mod serde;
pub mod syntax;
pub mod error;

pub use ordered_float::serialize_ordered_float;
pub use ordered_float::deserialize_ordered_float;
pub use error::SerializationError;
