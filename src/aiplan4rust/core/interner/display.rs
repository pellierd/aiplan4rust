//! Module `interner_display`
//!
//! This module defines traits for formatting values that require a `StringInterner`.
//!
//! It provides two related traits:
//! 1. [`InternerDisplay`] – for types that use an **external** interner when formatting.
//! 2. [`SelfInternerDisplay`] – for types that have their own **internal** interner.
//!
//! These traits allow rendering values into strings while resolving interned identifiers,
//! with convenient methods to obtain a `String` directly or via `Result`.

use crate::aiplan4rust::core::interner::SymbolInterner;
use std::fmt::{self, Write};

/// A trait for displaying a value with the help of an external `StringInterner`.
///
/// This is useful for types that need to resolve identifiers or other
/// interned strings when formatting themselves.
///
/// # Example
///
/// ```
/// impl DisplayWithInterner for MyType {
///     fn fmt_with(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
///         write!(f, "{}", interner.resolve(self.id))
///     }
/// }
/// ```
pub trait InternerDisplay {
    /// Formats the value using the given [`SymbolInterner`] and the provided formatter.
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &SymbolInterner,
    ) -> fmt::Result;

    /// Attempts to format the value into a [`String`] using the given [`SymbolInterner`].
    ///
    /// This version returns a `Result` and does not panic.
    ///
    /// # Example
    ///
    /// ```
    /// let s = my_value.try_to_string_with_interner(&interner)?;
    /// ```
    fn try_to_string_with_interner(&self, interner: &SymbolInterner) -> Result<String, fmt::Error>
    where
        Self: Sized,
    {
        let mut s = String::new();
        write!(
            &mut s,
            "{}",
            InternerDisplayWrapper {
                value: self,
                interner
            }
        )?;
        Ok(s)
    }

    /// Convenience method that formats the value into a [`String`] using the given
    /// [`SymbolInterner`].
    ///
    /// This method panics if formatting fails. Prefer [`try_to_string_with_interner`] if you want
    /// to handle errors explicitly.
    ///
    /// # Panics
    ///
    /// Panics if formatting into the string fails.
    ///
    /// # Example
    ///
    /// ```
    /// let s = my_value.to_string_with_interner(&interner);
    /// ```
    fn to_string_with_interner(&self, interner: &SymbolInterner) -> String
    where
        Self: Sized,
    {
        self.try_to_string_with_interner(interner)
            .expect("Formatting into String failed")
    }
}

/// Wrapper used to implement [`std::fmt::Display`] by delegating to [`InternerDisplay`].
pub struct InternerDisplayWrapper<'a, T: ?Sized> {
    pub value: &'a T,
    pub interner: &'a SymbolInterner,
}

impl<'a, T: InternerDisplay + ?Sized> fmt::Display for InternerDisplayWrapper<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_with_interner(f, self.interner)
    }
}

/// Trait for types that have an internal `StringInterner` and can render themselves.
///
/// Types implementing this trait are responsible for formatting themselves
/// using their own interner, typically resolving interned identifiers
/// and producing human-readable syntax strings.
///
/// # Example
///
/// ```rust
/// use std::fmt;
/// use crate::aiplan4rust::interner::StringInterner;
/// use crate::aiplan4rust::syntax_display::SelfInternerDisplay;
///
/// struct MyType {
///     id: usize,
///     interner: StringInterner,
/// }
///
/// impl MyType {
///     fn interner(&self) -> &StringInterner {
///         &self.interner
///     }
/// }
///
/// impl SelfInternerDisplay for MyType {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "{}", self.interner().resolve(self.id))
///     }
/// }
///
/// let value = MyType { id: 1, interner: StringInterner::new() };
/// let s = value.to_string();
/// println!("{}", s);
/// ```
pub trait SelfInternerDisplay {
    /// Formats the value using its internal `StringInterner`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure of the formatting.
    fn fmt_interner(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    /// Attempts to format the value into a `String` using its internal interner.
    ///
    /// This method returns a `Result` with the formatted string or a formatting error.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = my_value.try_to_string()?;
    /// ```
    fn try_to_string(&self) -> Result<String, fmt::Error>
    where
        Self: Sized,
    {
        let mut s = String::new();
        write!(&mut s, "{}", SelfInternerDisplayWrapper { value: self })?;
        Ok(s)
    }

    /// Formats the value into a `String` using its internal interner.
    ///
    /// This method panics if formatting fails.
    ///
    /// # Panics
    ///
    /// Panics if writing to the internal string fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = my_value.to_string();
    /// ```
    fn to_string(&self) -> String
    where
        Self: Sized,
    {
        self.try_to_string()
            .expect("Failed to render SelfInternerDisplay")
    }
}

/// Wrapper struct used to implement [`std::fmt::Display`] for any typing implementing [`SelfInternerDisplay`].
///
/// This allows using `write!` and other formatting macros with `SelfInternerDisplay`.
pub struct SelfInternerDisplayWrapper<'a, T: ?Sized> {
    /// Reference to the value to display.
    pub value: &'a T,
}

impl<'a, T: SelfInternerDisplay + ?Sized> fmt::Display for SelfInternerDisplayWrapper<'a, T> {
    /// Delegates the formatting to the `fmt` method of [`SelfInternerDisplay`].
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_interner(f)
    }
}
