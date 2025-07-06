use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::syntax::PlanningSyntaxDisplay;
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

/// Implements the standard `Display` trait for `Ident`.
///
/// This implementation formats the `Ident` by displaying its internal numeric
/// `value` prefixed with a `#` symbol. This representation is mainly for debugging
/// or simple identification purposes.
///
/// # Example
///
/// ```
/// let ident = Ident { value: 42 };
/// assert_eq!(format!("{}", ident), "#42");
/// ```
impl std::fmt::Display for Ident {
    /// Formats the identifier as `#<value>`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
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
    /// Formats the `Ident` by resolving its interned string using the given `StringInterner`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `interner` - The string interner to resolve the identifier.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt_with(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        if let Some(name) = interner.resolve(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", StringInterner::UNKNOWN_INTERNED_STRING)
        }
    }
}

/// Implements the `PlanningSyntaxDisplay` trait for `Ident`.
///
/// This implementation formats the `Ident` by using the provided
/// `StringInterner` to resolve the interned string associated with its `value`.
///
/// If the interner cannot resolve the identifier, it writes a placeholder string
/// in the format `<uninterned:{value}>` instead.
///
/// # Example
///
/// ```
/// let ident = Ident { value: 42 };
/// ident.fmt_planning(&mut formatter, &interner, 1)?;
/// ```
impl PlanningSyntaxDisplay for Ident {
    /// Formats the `Ident` using the given formatter and interner,
    /// applying indentation according to the `indent` level.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `interner` - The string interner used for resolving the identifier.
    /// * `indent` - The indentation level.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt_planning_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;

        if let Some(name) = interner.resolve(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "<uninterned:{}>", *self)
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
