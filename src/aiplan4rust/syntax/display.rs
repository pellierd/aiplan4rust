use std::fmt::Write;  // <-- Ajoute cet import
use crate::aiplan4rust::interner::StringInterner;

pub trait PlanningSyntaxDisplay {
    /// Number of characters per indentation level.
    fn indent_width() -> usize {
        2
    }

    /// Character used for indentation.
    fn indent_char() -> char {
        ' '
    }

    /// Builds the indentation string for a given level.
    fn make_indent(level: usize) -> String {
        let total = level * Self::indent_width();
        std::iter::repeat(Self::indent_char())
            .take(total)
            .collect()
    }

    /// Formats the value using the given [`StringInterner`] and the provided formatter.
    /// Now includes indent level.
    fn fmt_planning_syntax_with_indent(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> std::fmt::Result;

    /// Formats the value using the given [`StringInterner`] and the provided formatter.
    /// Now includes indent level.
    fn fmt_planning_syntax(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> std::fmt::Result {
        self.fmt_planning_syntax_with_indent(f, interner, 0)
    }

    /// Attempts to format into a String with a given indent.
    fn try_to_planning_string_with_indent(
        &self,
        interner: &StringInterner,
        indent: usize,
    ) -> Result<String, std::fmt::Error>
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
                indent,
            }
        )?;
        Ok(s)
    }

    /// Convenience method: panics if formatting fails.
    fn to_planning_string_with_indent(
        &self,
        interner: &StringInterner,
        indent: usize,
    ) -> String
    where
        Self: Sized,
    {
        self.try_to_planning_string_with_indent(interner, indent)
            .expect("Formatting into syntax string failed")
    }

    fn try_to_planning_string(
        &self,
        interner: &StringInterner,
    ) -> Result<String, std::fmt::Error>
    where
        Self: Sized,
    {
        self.try_to_planning_string_with_indent(interner, 0)
    }

    fn to_planning_string(
        &self,
        interner: &StringInterner,
    ) -> String
    where
        Self: Sized,
    {
        self.to_planning_string_with_indent(interner, 0)
    }
}

pub struct PlanningDisplayWrapper<'a, T: ?Sized> {
    pub value: &'a T,
    pub interner: &'a StringInterner,
    pub indent: usize,
}

impl<'a, T: PlanningSyntaxDisplay + ?Sized> std::fmt::Display for PlanningDisplayWrapper<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt_planning_syntax_with_indent(f, self.interner, self.indent)
    }
}
