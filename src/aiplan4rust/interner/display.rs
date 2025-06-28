//! Defines the `DisplayWithInterner` trait for formatting values that need an interner.
//!
//! This allows types to render themselves into strings while resolving interned identifiers.

use std::fmt::{self, Write};
use crate::aiplan4rust::interner::StringInterner;

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
pub trait DisplayWithInterner {
    /// Formats the value using the given [`StringInterner`] and the provided formatter.
    fn fmt_with(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result;

    /// Attempts to format the value into a [`String`] using the given [`StringInterner`].
    ///
    /// This version returns a `Result` and does not panic.
    ///
    /// # Example
    ///
    /// ```
    /// let s = my_value.try_to_string_with_interner(&interner)?;
    /// ```
    fn try_to_string_with_interner(
        &self,
        interner: &StringInterner,
    ) -> Result<String, fmt::Error>
    where
        Self: Sized,
    {
        let mut s = String::new();
        write!(
            &mut s,
            "{}",
            DisplayWithInternerWrapper {
                value: self,
                interner
            }
        )?;
        Ok(s)
    }

    /// Convenience method that formats the value into a [`String`] using the given
    /// [`StringInterner`].
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
    fn to_string_with_interner(
        &self,
        interner: &StringInterner,
    ) -> String
    where
        Self: Sized,
    {
        self.try_to_string_with_interner(interner)
            .expect("Formatting into String failed")
    }
}

/// Wrapper used to implement [`std::fmt::Display`] by delegating to [`DisplayWithInterner`].
pub struct DisplayWithInternerWrapper<'a, T: ?Sized> {
    pub value: &'a T,
    pub interner: &'a StringInterner,
}

impl<'a, T: DisplayWithInterner + ?Sized> fmt::Display
for DisplayWithInternerWrapper<'a, T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_with(f, self.interner)
    }
}
