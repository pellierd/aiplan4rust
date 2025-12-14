//! This module provides serialization-related utilities and types.
//!
//! It includes submodules for different serialization formats (`format`),
//! extensions to serialization behaviors (`extension`), and core serialization
//! traits and implementations (`serializable`).
//!
//! The main exports from this module are:
//! - `SerdeFormat`: abstraction over serialization formats.
//! - `SerdeExtension`: additional capabilities or customizations for serialization.
//! - `SerdeSerializable`: core trait for serializable types.
pub mod serializable;
pub mod format;
pub mod extension;
pub mod header;

pub use format::Format as SerdeFormat;
pub use extension::Extension as SerdeExtension;
pub use serializable::Serializable as SerdeSerializable;
pub use header::Header as SerdeHeader;
