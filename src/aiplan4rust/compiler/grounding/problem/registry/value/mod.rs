//! Management of object domains and type-safe value registries.
//!
//! This module provides the infrastructure to map PDDL types to their respective
//! object domains. It handles the complexity of inheritance, multiple inheritance,
//! and flattened memory layouts to provide high-performance grounding.
//!
//! # Architecture
//!
//! * [`ValueRegistry`]: The central structure that stores flattened object domains
//!     and provides $O(1)$ access to type-specific slices.
//! * [`error`]: Error types related to registry construction and validation,
//!     including cycle detection and type mismatch.
//! * [`range`]: Internal primitives for mapping types to specific memory segments
//!     within the registry's global buffer.
//!
//! # Grounding Workflow
//!
//! 1.  **Extraction**: The type hierarchy is converted into an adjacency graph.
//! 2.  **Mapping**: Constant objects are assigned to their declared primitive types.
//! 3.  **Flattening**: Subtype objects are propagated up to their ancestors, ensuring 
//!     that a parent type's domain is a contiguous superset of its children's domains.
//! 4.  **Retrieval**: The engine retrieves `&[ObjectId]` slices to generate 
//!     the cartesian product of action parameters.

pub mod registry;
#[cfg(test)]
mod registry_tests;
mod range;
pub mod error;

pub use registry::ValueRegistry;
