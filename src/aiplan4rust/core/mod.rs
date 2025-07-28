//! Core abstractions and data structures for aiplan4rust.
//!
//! This module provides the foundational components shared across the AI syntax library,
//! including arena-based tree structures, syntax node traits, and unique identifiers.
//!
//! # Modules
//!
//! - [`arena`]: Implements the arena memory model and associated types such as nodes, errors, and IDs.
//!
//! # Purpose
//!
//! The `core` module centralizes the essential building blocks that underpin the
//! representation and manipulation of syntax trees and syntax graphs within aiplan4rust.
//! It provides safe, efficient, and ergonomic abstractions that other modules build upon.
//!
//! # Future expansion
//!
//! Additional core utilities, traits, and data structures common to multiple parts of the library
//! will be added here as the project evolves.
pub mod arena;
