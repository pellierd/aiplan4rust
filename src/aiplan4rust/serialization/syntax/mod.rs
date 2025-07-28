//! Module providing traits and types for serialization and deserialization
//! of planning-related data structures.
//!
//! This module includes:
//! - [`serializable`]: Trait definitions for serializing and deserializing syntax structures.
//! - [`format`]: Supported file formats (e.g., PDDL, HDDL).
//! - [`extension`]: Supported file extensions for planning files.
//!
//! The public re-exports provide convenient aliases for working with these types:
//! - [`PlanningSerializable`] trait for generic serialization support.
//! - [`PlanningFormat`] enum for supported serialization formats.
//! - [`PlanningExtension`] enum for supported file extensions.
pub mod serializable;
pub mod format;
pub mod extension;

pub use serializable::Serializable as PlanningSerializable;
pub use format::Format as PlanningFormat;
pub use extension::Extension as PlanningExtension;
