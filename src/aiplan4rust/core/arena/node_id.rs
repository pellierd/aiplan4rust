//! Unique node identifiers for arena-managed tree structures.
//!
//! This module defines the [`NodeId`] type_checker, which wraps a `usize` index to uniquely
//! identify nodes stored within an arena (a contiguous node storage).
//!
//! # Key Features
//!
//! - Provides strong typing and clarity by wrapping raw indices in a distinct type_checker.
//! - Defines a sentinel invalid ID (`usize::MAX`) for easy validation checks.
//! - Implements common traits for copying, hashing, serialization, and debugging.
//! - Supports (de)serialization as strings for better readability in serialized forms.
//! - Offers convenient conversion to/from `usize`.
//!
//! # Usage
//!
//! `NodeId` is the primary way to refer to nodes in arena-based trees, ensuring
//! type_checker safety and preventing accidental misuse of raw indices.
//!
//! Typical operations include creating new IDs, validating their correctness,
//! and converting them to and from raw indices or serialized forms.
//!
//! # Examples
//!
//! ```rust
//! use crate::NodeId;
//!
//! let id = NodeId::new(42);
//! assert!(id.is_valid());
//! assert_eq!(id.as_usize(), 42);
//!
//! let invalid = NodeId::default();
//! assert!(!invalid.is_valid());
//! ```
//!
//! # Serialization
//!
//! `NodeId` serializes as a string representation of its numeric value and
//! deserializes from the same, enhancing readability in JSON or other text formats.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;

/// A unique identifier for nodes within an arena.
///
/// `NodeId` wraps a `usize` index used to uniquely identify nodes in
/// an arena-managed tree structure. It provides type_checker safety and utility
/// methods for working with node identifiers.
///
/// # Sentinel value
///
/// The value `usize::MAX` is reserved as a sentinel representing an invalid
/// or uninitialized `NodeId`. This allows easy detection of invalid IDs.
///
/// # Examples
///
/// ```
/// let id = NodeId::new(42);
/// assert_eq!(id.as_usize(), 42);
/// assert!(id.is_valid());
///
/// let invalid = NodeId::default();
/// assert!(!invalid.is_valid());
/// ```
///
/// # Traits
///
/// Implements `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Serialize`, and `Deserialize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    /// The underlying integer value representing the node identifier.
    ///
    /// The sentinel value `usize::MAX` indicates an invalid ID.
    pub value: usize,
}

impl Default for NodeId {
    /// Returns an invalid `NodeId` with the sentinel value `usize::MAX`.
    fn default() -> Self {
        NodeId { value: usize::MAX }
    }
}

impl NodeId {
    /// Creates a new `NodeId` wrapping the given `usize` value.
    ///
    /// # Arguments
    ///
    /// * `value` - The integer value to use as the node identifier.
    ///
    /// # Returns
    ///
    /// A new `NodeId` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = NodeId::new(7);
    /// assert_eq!(id.as_usize(), 7);
    /// ```
    pub const fn new(value: usize) -> Self {
        NodeId { value }
    }

    /// Returns the raw `usize` value of this `NodeId`.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = NodeId::new(10);
    /// assert_eq!(id.as_usize(), 10);
    /// ```
    pub fn as_usize(&self) -> usize {
        self.value
    }

    /// Returns `true` if the `NodeId` is valid (not the sentinel invalid value).
    ///
    /// # Examples
    ///
    /// ```
    /// let valid = NodeId::new(5);
    /// let invalid = NodeId::default();
    ///
    /// assert!(valid.is_valid());
    /// assert!(!invalid.is_valid());
    /// ```
    pub fn is_valid(&self) -> bool {
        self.value != usize::MAX
    }
}

// Serialization of NodeId as string
impl Serialize for NodeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_str(&self.value.to_string())
    }
}

// Deserialization of NodeId from string
impl<'de> Deserialize<'de> for NodeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        let value = s.parse::<usize>().map_err(D::Error::custom)?;
        Ok(NodeId { value })
    }
}

impl std::fmt::Display for NodeId {
    /// Formats the `NodeId` for user-friendly display.
    ///
    /// Shows `#<value>` if valid, or `#invalid` if not.
    ///
    /// # Examples
    ///
    /// ```
    /// let valid = NodeId::new(3);
    /// let invalid = NodeId::default();
    /// assert_eq!(format!("{}", valid), "#3");
    /// assert_eq!(format!("{}", invalid), "#invalid");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid() {
            write!(f, "#{}", self.value)
        } else {
            write!(f, "#invalid")
        }
    }
}

impl From<usize> for NodeId {
    /// Converts a `usize` into a `NodeId`.
    ///
    /// # Examples
    ///
    /// ```
    /// let id: NodeId = 10usize.into();
    /// assert_eq!(id.as_usize(), 10);
    /// ```
    fn from(value: usize) -> Self {
        NodeId::new(value)
    }
}

impl From<NodeId> for usize {
    /// Converts a `NodeId` into its underlying `usize` value.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = NodeId::new(5);
    /// let raw: usize = id.into();
    /// assert_eq!(raw, 5);
    /// ```
    fn from(id: NodeId) -> usize {
        id.value
    }
}
