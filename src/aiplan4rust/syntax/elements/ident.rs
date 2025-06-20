use serde::{Deserialize, Serialize};

/// Represents a unique identifier as an unsigned integer.
///
/// This struct wraps a `usize` that acts as an index or identifier in contexts
/// such as a string interner. It provides methods to access the underlying value
/// and enables type-safe handling of identifiers.
///
/// The `Default` implementation uses `usize::MAX` as a special sentinel value to
/// indicate an uninitialized or invalid identifier. Since `usize` is an unsigned
/// type and cannot hold negative values, `usize::MAX` (the maximum possible
/// value for a `usize`) is chosen as a stand-in for "no valid id".
///
/// Users should be aware of this sentinel when checking the validity of an
/// `Ident`. For example, an `Ident` with value `usize::MAX` can be treated as
/// "empty" or "unset".
///
/// # Example
///
/// ```
/// let id = Ident::new(42);
/// println!("Identifier: {}", id);
/// let raw_value = id.as_usize();
/// assert_eq!(raw_value, 42);
/// ```
///
/// # Default behavior
///
/// ```
/// let default_id = Ident::default();
/// assert_eq!(default_id.value, usize::MAX);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ident {
    /// The integer value representing the identifier.
    pub value: usize,
}

impl Default for Ident {
    /// Returns an `Ident` with the sentinel value `usize::MAX`.
    ///
    /// This value is used to represent an invalid or uninitialized identifier,
    /// since `usize` cannot hold negative values.
    fn default() -> Self {
        Ident { value: usize::MAX }
    }
}

impl Ident {
    /// Creates a new identifier from a `usize` value.
    ///
    /// # Arguments
    ///
    /// * `value` - The integer to be used as the identifier.
    ///
    /// # Returns
    ///
    /// A new `Ident` instance.
    pub const fn new(value: usize) -> Self {
        Ident { value }
    }

    /// Returns the underlying integer value of the identifier.
    ///
    /// # Example
    ///
    /// ```
    /// let id = Ident::new(7);
    /// assert_eq!(id.as_usize(), 7);
    /// ```
    pub fn as_usize(&self) -> usize {
        self.value
    }
}

impl std::fmt::Display for Ident {
    /// Formats the identifier for display.
    ///
    /// Displays as `Ident(<value>)`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ident({})", self.value)
    }
}

impl From<usize> for Ident {
    /// Converts a `usize` into an `Ident`.
    ///
    /// # Example
    ///
    /// ```
    /// let id: Ident = 10usize.into();
    /// assert_eq!(id.as_usize(), 10);
    /// ```
    fn from(value: usize) -> Self {
        Ident::new(value)
    }
}

impl From<Ident> for usize {
    /// Converts an `Ident` into a `usize`.
    ///
    /// # Example
    ///
    /// ```
    /// let id = Ident::new(5);
    /// let raw_value: usize = id.into();
    /// assert_eq!(raw_value, 5);
    /// ```
    fn from(ident: Ident) -> usize {
        ident.value
    }
}
