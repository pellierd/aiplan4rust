use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use ordered_float::OrderedFloat;
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::arena::{ArenaNode, NodeContent};
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, Ident, Optimization};
use crate::aiplan4rust::semantic::symbol::SymbolRef;
use crate::aiplan4rust::syntax::core::SyntaxTree;

pub trait SyntaxNode: ArenaNode {

    /// Remaps identifiers inside the syntax’s content according to the given map.
    ///
    /// This is useful for operations like renaming or merging scopes.
    ///
    /// # Arguments
    ///
    /// - `map`: A `HashMap` mapping old identifiers to new identifiers.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>);

    // Delegation methods to the syntax’s content, allowing convenient extraction
    // of specific semantic types without manually matching on content.

    /// Returns the identifier if present in the syntax’s content.
    fn as_ident(&self) -> Option<Ident> {
        self.content().as_ident()
    }

    /// Returns the floating-point literal if present in the syntax’s content.
    fn as_float(&self) -> Option<OrderedFloat<f64>> {
        self.content().as_float()
    }

    /// Returns the binary comparison operator if present in the syntax’s content.
    fn as_binary_comp(&self) -> Option<BinaryComp> {
        self.content().as_binary_comp()
    }

    /// Returns the assignment operator if present in the syntax’s content.
    fn as_assign_op(&self) -> Option<AssignOp> {
        self.content().as_assign_op()
    }

    /// Returns the arithmetic operator if present in the syntax’s content.
    fn as_arithmetic_op(&self) -> Option<ArithmeticOp> {
        self.content().as_arithmetic_op()
    }

    /// Returns the optimization directive if present in the syntax’s content.
    fn as_optimization(&self) -> Option<Optimization> {
        self.content().as_optimization()
    }

    fn as_symbol_ref(&self) -> Result<Option<SymbolRef>, AiplanError>;

    // Try-extraction methods that return Result for better error handling.

    /// Attempts to extract an identifier from the syntax’s content.
    fn try_ident(&self) -> Result<Ident, AiplanError> {
        self.content().try_ident()
    }

    /// Attempts to extract a floating-point literal from the syntax’s content.
    fn try_float(&self) -> Result<OrderedFloat<f64>, AiplanError> {
        self.content().try_float()
    }

    /// Attempts to extract a binary comparison operator from the syntax’s content.
    fn try_binary_comp(&self) -> Result<BinaryComp, AiplanError> {
        self.content().try_binary_comp()
    }

    /// Attempts to extract an assignment operator from the syntax’s content.
    fn try_assign_op(&self) -> Result<AssignOp, AiplanError> {
        self.content().try_assign_op()
    }

    /// Attempts to extract an arithmetic operator from the syntax’s content.
    fn try_arithmetic_op(&self) -> Result<ArithmeticOp, AiplanError> {
        self.content().try_arithmetic_op()
    }

    /// Attempts to extract an optimization directive from the syntax’s content.
    fn try_optimization(&self) -> Result<Optimization, AiplanError> {
        self.content().try_optimization()
    }
    fn try_symbol_ref(&self) -> Result<SymbolRef, AiplanError> {
        self.as_symbol_ref()?.ok_or_else(|| AiplanError::InternalError("Not a SymbolRef".to_string()))
    }

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
