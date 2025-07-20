use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::syntax::core::SyntaxTree;
use crate::aiplan4rust::syntax::SyntaxDisplay;

pub trait SyntaxNode: ArenaNode {

    /// Formats the syntax with access to the arena and an interner.
    ///
    /// This method allows accessing other nodes in the arena,
    /// useful for displaying children or related information.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `arena` - Reference to the arena containing all nodes.
    /// * `interner` - Reference to the interner for resolving identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, arena: &SyntaxTree<Self>, interner: &StringInterner) -> fmt::Result
    where Self: Sized;

    /// Converts the syntax to a string using `fmt_with`.
    ///
    /// # Arguments
    ///
    /// * `arena` - Reference to the arena to fetch other nodes if needed.
    /// * `interner` - Reference to the interner for resolving identifiers.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = syntax.to_string_with_interner(&arena, &interner);
    /// println!("{}", s);
    /// ```
    fn to_string_with_interner(&self, arena: &SyntaxTree<Self>, interner: &StringInterner) -> String
    where Self: Sized {
        struct DisplayWrapper<'a, T: SyntaxNode> {
            node: &'a T,
            arena: &'a SyntaxTree<T>,
            interner: &'a StringInterner,
        }

        impl<'a, T: SyntaxNode> fmt::Display for DisplayWrapper<'a, T> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result  {
                self.node.fmt_with_interner(f, self.arena, self.interner)
            }
        }

        format!("{}", DisplayWrapper { node: self, arena, interner })
    }


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

    /// Formats the syntax using a specific planning syntax style,
    /// applying the given indentation level.
    ///
    /// This method is similar to `fmt_with`, but formats the syntax
    /// according to a custom grammar or syntax conventions (for example,
    /// PDDL-like syntax). Implementors should respect the provided
    /// `indent` level when producing their output.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `arena` - A reference to the arena containing all nodes.
    /// * `interner` - A reference to the `StringInterner` for resolving identifiers.
    /// * `indent` - The indentation level (number of indent units).
    ///
    /// # Errors
    ///
    /// Returns a [`fmt::Error`] if writing to the formatter fails.
    fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        arena: &SyntaxTree<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result
    where
        Self: Sized;

    /// Formats the syntax using a specific planning syntax style with no indentation.
    ///
    /// This is a convenience method that simply calls
    /// [`fmt_planning_syntax_with_indent`] with an indent level of 0.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write to.
    /// * `arena` - A reference to the arena containing all nodes.
    /// * `interner` - A reference to the `StringInterner` for resolving identifiers.
    ///
    /// # Errors
    ///
    /// Returns a [`fmt::Error`] if writing to the formatter fails.
    fn fmt_syntax(
        &self,
        f: &mut Formatter<'_>,
        arena: &SyntaxTree<Self>,
        interner: &StringInterner,
    ) -> fmt::Result
    where
        Self: Sized,
    {
        self.fmt_syntax_with_indent(f, arena, interner, 0)
    }


    /// Converts the syntax to a string using a specific planning syntax format,
    /// applying the given indentation level.
    ///
    /// This method wraps the syntax in a temporary formatter to produce
    /// a syntax-oriented string representation using `fmt_planning`.
    ///
    /// # Arguments
    ///
    /// * `arena` - A reference to the arena containing the arena of nodes.
    /// * `interner` - A reference to the `StringInterner` used to resolve identifiers.
    /// * `indent` - The indentation level (number of indent units to apply).
    ///
    /// # Returns
    ///
    /// A `String` containing the formatted syntax representation of the syntax.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = syntax.to_planning_syntax_with_indent(&arena, &interner, 2);
    /// println!("{}", s);
    /// ```
    fn to_syntax_with_indent(
        &self,
        arena: &SyntaxTree<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> String
    where
        Self: Sized,
    {
        struct PlanningSyntaxDisplayWrapper<'a, T: SyntaxNode> {
            node: &'a T,
            arena: &'a SyntaxTree<T>,
            interner: &'a StringInterner,
            indent: usize,
        }

        impl<'a, T: SyntaxNode> fmt::Display
        for PlanningSyntaxDisplayWrapper<'a, T>
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.node.fmt_syntax_with_indent(f, self.arena, self.interner, self.indent)
            }
        }

        format!(
            "{}",
            PlanningSyntaxDisplayWrapper {
                node: self,
                arena,
                interner,
                indent
            }
        )
    }

    /// Converts the syntax to a string using the specific syntax formatting without indentation.
    ///
    /// This simply calls `to_planning_syntax_with_indent` with an indent level of 0.
    ///
    /// # Arguments
    ///
    /// * `arena` - Reference to the arena containing the nodes.
    /// * `interner` - Reference to the interner for resolving identifiers.
    ///
    /// # Example
    ///
    /// ```rust
    /// let s = syntax.to_planning_syntax(&arena, &interner);
    /// println!("{}", s);
    /// ```
    fn to_syntax_string(
        &self,
        arena: &SyntaxTree<Self>,
        interner: &StringInterner,
    ) -> String
    where
        Self: Sized,
    {
        self.to_syntax_with_indent(arena, interner, 0)
    }
}
