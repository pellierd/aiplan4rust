//! This module defines the `Id` trait and related identifier types.
//!
//! The `Id` trait represents interned identifiers backed by a `usize` value.
//! It provides common methods to create, validate, and convert identifiers.
//!
//! Types implementing `Id` can be used safely for interning and indexing.

/// Trait representing a general interned identifier.
///
/// This trait assumes the implementing type wraps a unique `usize` value,
/// typically used for interning or indexing purposes.
///
/// Types implementing this trait should provide a way to create a new instance
/// from a `usize` and to access the underlying `usize` value.
/// Additionally, the trait provides default methods for checking validity
/// based on a sentinel invalid value, and conversion helpers.
///
/// # Trait bounds
///
/// Implementors must be `Sized`, `Copy`, `PartialEq`, `Eq`, and `Hash`.
///
/// # Examples
///
/// ```
/// struct MyId(usize);
///
/// impl Id for MyId {
///     fn value(&self) -> usize { self.0 }
///     fn new(value: usize) -> Self { MyId(value) }
/// }
/// ```
pub trait Id: Sized + Copy + PartialEq + Eq + std::hash::Hash + From<usize> {
    /// Returns the internal `usize` value of the identifier.
    ///
    /// # Returns
    ///
    /// The `usize` value representing this identifier.
    fn value(&self) -> usize;

    /// Creates a new instance of the identifier from the given `usize` value.
    ///
    /// # Arguments
    ///
    /// * `value` - A `usize` representing the unique identifier.
    ///
    /// # Returns
    ///
    /// A new instance of the implementing type wrapping the provided value.
    fn new(value: usize) -> Self;

    /// Returns the sentinel value representing an invalid identifier.
    ///
    /// The default implementation returns `usize::MAX`.
    ///
    /// # Returns
    ///
    /// The `usize` value that signifies an invalid or uninitialized identifier.
    fn invalid_value() -> usize {
        usize::MAX
    }

    /// Checks whether the identifier is valid.
    ///
    /// An identifier is considered valid if its internal value is different
    /// from the sentinel invalid value.
    ///
    /// # Returns
    ///
    /// `true` if the identifier is valid, `false` otherwise.
    fn is_valid(&self) -> bool {
        self.value() != Self::invalid_value()
    }

    /// Returns the underlying `usize` value of the identifier.
    ///
    /// This is a convenience method equivalent to `value()`.
    ///
    /// # Returns
    ///
    /// The `usize` value wrapped by this identifier.
    fn as_usize(&self) -> usize {
        self.value()
    }
}
