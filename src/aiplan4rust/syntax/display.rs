//! Defines the `DisplaySyntax` trait for formatting values in user-facing syntax,
//! potentially using an interner to resolve identifiers.

use std::fmt::{self, Formatter, Write};
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};

/// A trait for displaying a value in its concrete syntax,
/// resolving any interned identifiers as needed.
pub trait DisplaySyntax {
    /// Formats the value using the given [`StringInterner`] and the provided formatter.
    fn fmt_syntax(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result;

    /// Attempts to format the value into a [`String`] using the given [`StringInterner`].
    ///
    /// This version returns a `Result` and does not panic.
    fn try_to_syntax_string(
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
            DisplaySyntaxWrapper {
                value: self,
                interner,
            }
        )?;
        Ok(s)
    }

    /// Convenience method that formats the value into a [`String`] using the given
    /// [`StringInterner`].
    ///
    /// This method panics if formatting fails. Prefer [`try_to_syntax_string`] if you want
    /// to handle errors explicitly.
    ///
    /// # Panics
    ///
    /// Panics if formatting into the string fails.
    fn to_syntax_string(
        &self,
        interner: &StringInterner,
    ) -> String
    where
        Self: Sized,
    {
        self.try_to_syntax_string(interner)
            .expect("Formatting into syntax string failed")
    }
}

/// A wrapper used to implement [`std::fmt::Display`] by delegating to [`DisplaySyntax`].
pub struct DisplaySyntaxWrapper<'a, T: ?Sized> {
    pub value: &'a T,
    pub interner: &'a StringInterner,
}

impl<'a, T: DisplaySyntax + ?Sized> fmt::Display
for DisplaySyntaxWrapper<'a, T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_syntax(f, self.interner)
    }
}
