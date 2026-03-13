/// This module provides a lazy, pruning-capable iterator for variable bindings.
///
/// In the context of grounding, a `BindingsIterator` allows for the exploration 
/// of the Cartesian product of multiple object domains. It is specifically 
/// designed to be efficient by avoiding unnecessary allocations and supporting 
/// search-space pruning.
///
/// # Submodules
/// - `iterator`: Contains the core logic for the Cartesian product traversal.
/// - `error`: Defines error types related to combinatorial explosions and overflows.
pub mod iter;

/// Error types specific to the binding iteration process.
pub mod error;

#[cfg(test)]
mod iter_tests;

pub use iter::BindingsIterator;
pub use error::BindingsIteratorError;
