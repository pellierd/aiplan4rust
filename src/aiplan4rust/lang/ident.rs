use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::syntax::PlanningDisplay;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;

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
#[serde(transparent)]
pub struct Ident {
    /// The integer value representing the identifier.
    value: usize,
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

    /// Returns `true` if this identifier holds a valid (initialized) value.
    ///
    /// # Example
    /// ```
    /// let ident = Ident::default();
    /// assert!(!ident.is_valid());
    ///
    /// let ident = Ident { value: 42 };
    /// assert!(ident.is_valid());
    /// ```
    pub fn is_valid(&self) -> bool {
        self.value != usize::MAX
    }
}

impl std::fmt::Display for Ident {
    /// Formats the identifier for display.
    ///
    /// This implementation shows the identifier as `#<value>`,
    /// where `<value>` is the internal numeric representation.
    ///
    /// # Example
    ///
    /// ```
    /// let ident = Ident { value: 42 };
    /// assert_eq!(format!("{}", ident), "#42");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.value)
    }
}

/// Implements the `DisplayWithInterner` trait for `Ident`.
///
/// This allows formatting an `Ident` by resolving its internal
/// `usize` value to a string using the provided `StringInterner`.
///
/// If the interner cannot resolve the identifier, a fallback
/// constant string `UNKNOWN_INTERNED_STRING` is displayed instead.
///
/// # Example
///
/// ```
/// let ident = Ident { value: 42 };
/// let interner = StringInterner::new();
/// // Assuming interner has some strings interned
/// let s = ident.to_string_with_interner(&interner);
/// ```
impl DisplayWithInterner for Ident {
    fn fmt_with(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        if let Some(name) = interner.resolve(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", StringInterner::UNKNOWN_INTERNED_STRING)
        }
    }
}

/// Implements the `DisplaySyntax` trait for `Ident`.
///
/// This uses the provided `StringInterner` to resolve the interned string
/// corresponding to the identifier's `value`.
///
/// If the interner cannot resolve the value, it falls back to displaying
/// the raw `usize` value.
///
/// # Example
///
/// ```
/// let ident = Ident { value: 42 };
/// let s = ident.fmt_syntax(&mut formatter, &interner)?;
/// ```
impl PlanningDisplay for Ident {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        if let Some(name) = interner.resolve(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", StringInterner::UNKNOWN_INTERNED_STRING)
        }
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
