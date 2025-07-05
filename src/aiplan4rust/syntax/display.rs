//! Defines the `DisplaySyntax` trait for formatting values in user-facing syntax,
//! potentially using an interner to resolve identifiers.

use std::fmt::{self, Write};
use crate::aiplan4rust::interner::StringInterner;

/// A trait for displaying a value in its concrete syntax,
/// resolving any interned identifiers as needed.
pub trait PlanningSyntaxDisplay {
    /// Formats the value using the given [`StringInterner`] and the provided formatter.
    fn fmt_planning(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result;

    /// Attempts to format the value into a [`String`] using the given [`StringInterner`].
    ///
    /// This version returns a `Result` and does not panic.
    fn try_to_planning_string(
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
            PlanningDisplayWrapper {
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
    fn to_planning_string(
        &self,
        interner: &StringInterner,
    ) -> String
    where
        Self: Sized,
    {
        self.try_to_planning_string(interner)
            .expect("Formatting into syntax string failed")
    }
}

/// A wrapper used to implement [`std::fmt::Display`] by delegating to [`PlanningSyntaxDisplay`].
pub struct PlanningDisplayWrapper<'a, T: ?Sized> {
    pub value: &'a T,
    pub interner: &'a StringInterner,
}

impl<'a, T: PlanningSyntaxDisplay + ?Sized> fmt::Display
for PlanningDisplayWrapper<'a, T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt_planning(f, self.interner)
    }
}
