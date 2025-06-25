use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;

/// A unique identifier for nodes in an arena.
///
/// This struct wraps a `usize` that serves as a unique index or ID for nodes
/// in a tree or arena structure. It provides type safety and utility methods
/// to work with node identifiers.
///
/// # Sentinel value
///
/// The value `usize::MAX` is reserved as a sentinel to represent an invalid
/// or uninitialized `NodeId`. This allows distinguishing between valid and
/// invalid IDs.
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
/// # Derives
///
/// Implements `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id {
    /// The integer value representing the node identifier.
    ///
    /// `usize::MAX` is used as an invalid sentinel value.
    pub value: usize,
}

impl Default for Id {
    /// Returns a default invalid `NodeId` with the sentinel value `usize::MAX`.
    fn default() -> Self {
        Id { value: usize::MAX }
    }
}

impl Id {
    /// The constant identifier for the root node in the arena.
    ///
    /// This constant represents the ID of the root node, which is always zero.
    /// It is used to access the root node within the arena.
    pub const ROOT_ID: Id = Id::new(0);

    /// Creates a new `NodeId` from a `usize` value.
    ///
    /// # Arguments
    ///
    /// * `value` - The integer value to use as the node identifier.
    ///
    /// # Returns
    ///
    /// A new `NodeId` wrapping the provided value.
    pub const fn new(value: usize) -> Self {
        Id { value }
    }

    /// Returns the underlying `usize` value of the `NodeId`.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = NodeId::new(7);
    /// assert_eq!(id.as_usize(), 7);
    /// ```
    pub fn as_usize(&self) -> usize {
        self.value
    }

    /// Checks whether the `NodeId` is valid (not equal to the sentinel `usize::MAX`).
    ///
    /// # Returns
    ///
    /// `true` if the ID is valid, `false` otherwise.
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

// Sérialisation en string
impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_str(&self.value.to_string())
    }
}

// Désérialisation depuis string
impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        let value = s.parse::<usize>().map_err(D::Error::custom)?;
        Ok(Id { value })
    }
}

impl std::fmt::Display for Id {
    /// Formats the `NodeId` for display purposes.
    ///
    /// Displays as `NodeId(<value>)` if valid, or `NodeId(<invalid>)` if not.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid() {
            write!(f, "NodeId({})", self.value)
        } else {
            write!(f, "NodeId(<invalid>)")
        }
    }
}

impl From<usize> for Id {
    /// Converts a `usize` into a `NodeId`.
    ///
    /// # Examples
    ///
    /// ```
    /// let id: NodeId = 10usize.into();
    /// assert_eq!(id.as_usize(), 10);
    /// ```
    fn from(value: usize) -> Self {
        Id::new(value)
    }
}

impl From<Id> for usize {
    /// Converts a `NodeId` back into a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = NodeId::new(5);
    /// let raw: usize = id.into();
    /// assert_eq!(raw, 5);
    /// ```
    fn from(id: Id) -> usize {
        id.value
    }
}
