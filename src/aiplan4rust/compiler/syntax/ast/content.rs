//! AST Node Content Representation
//!
//! This module defines [`Content`], an enum used to attach semantic or syntactic meaning
//! to an AST syntax. Each variant represents a concrete payload, such as an identifier,
//! floating-point value, or PDDL-specific operator (e.g., comparison or assignment).
//!
//! `Content` is a leaf element in the AST: it holds data but no arena structure.
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::syntax::Content;
//! use ordered_float::OrderedFloat;
//!
//! let value = Content::Float(OrderedFloat(3.14));
//! println!("{}", value); // Prints: 3.14
//! ```
//!
//! # Display with Context
//!
//! Identifiers are interned during parsing. To resolve them to human-readable strings,
//! use [`Content::display_with_context`] with a [`SymbolInterner`].
//!
//! ```rust
//! use aiplan4rust::syntax::{Content, StringInterner};
//!
//! let mut interner = StringInterner::default();
//! let id = interner.intern("move");
//! let content = Content::Ident(id);
//!
//! assert_eq!(content.display_with_context(&interner), "move");
//! ```

use crate::aiplan4rust::cli::io::serialization::{
    deserialize_ordered_float, serialize_ordered_float,
};
use crate::aiplan4rust::compiler::syntax::ast::tree::SyntaxContent;
use crate::aiplan4rust::compiler::syntax::ast::AstError;
use crate::aiplan4rust::compiler::syntax::{write_indent, SyntaxInternerDisplay};
use crate::aiplan4rust::support::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::support::lang::{
    ArithmeticOp, AssignOp, CompareOp, OptimizationOp, RemapSymbol, Requirement, SymbolId,
};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

/// Represents semantic content associated with an AST syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Content {
    /// No content (debug/empty syntax).
    #[default]
    None,

    /// Interned identifier (references a string in the [`SymbolInterner`]).
    Ident(SymbolId),

    /// Floating-point literal (wrapped in [`OrderedFloat`] for total ordering).
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Number(OrderedFloat<f64>),

    /// A requirement flag such as `:typing` or `:equality`.
    Requirement(Requirement),

    /// A comparison operator, e.g. `=`, `<`, `>`.
    CompareOp(CompareOp),

    /// An assignment operator, e.g. `assign`, `increase`.
    AssignOp(AssignOp),

    /// An arithmetic operator like `+`, `-`, `*`, `/`.
    ArithmeticOp(ArithmeticOp),

    /// An optimization directive such as `maximize` or `minimize`.
    OptimizationOp(OptimizationOp),
}

impl Content {
    /// Returns the requirement flag if this content is a `Requirement`.
    ///
    /// # Returns
    ///
    /// - `Some(Requirement)` if the content is a requirement.
    /// - `None` otherwise.
    pub fn as_requirement(&self) -> Option<Requirement> {
        match self {
            Content::Requirement(r) => Some(*r),
            _ => None,
        }
    }

    /// Returns the requirement flag if this content is a `Requirement`.
    ///
    /// # Errors
    ///
    /// Returns `AstError::NotARequirement` if the content is not a `Requirement`.
    pub fn try_requirement(&self) -> Result<Requirement, AstError> {
        match self {
            Content::Requirement(r) => Ok(*r),
            _ => Err(AstError::not_a_requirement()),
        }
    }

    /// Returns the content as an identifier if available.
    pub fn as_ident(&self) -> Option<SymbolId> {
        match self {
            Content::Ident(id) => Some(*id),
            _ => None,
        }
    }

    /// Attempts to extract an identifier from the content.
    ///
    /// Returns `Ok(Ident)` if successful or
    /// `Err(SyntaxTreeError::NotAnIdent)` if the content is not an identifier.
    pub fn try_ident(&self) -> Result<SymbolId, AstError> {
        self.as_ident().ok_or_else(|| AstError::not_a_symbol_id())
    }
}

/// Implements the standard `fmt::Display` trait for the `Content` enum.
///
/// This implementation provides a straightforward textual representation
/// of each variant without relying on interner-based resolution.
///
/// - `None` variant renders as an empty string.
/// - `Ident` variant formats its contained index directly (typically as `#<value>`).
/// - Other variants delegate to their own `Display` implementations.
///
/// This is suitable for general-purpose display where interned string lookup
/// is not needed or unavailable.
///
/// # Example
///
/// ```
/// let c = Content::Float(3.14);
/// println!("{}", c); // prints "3.14"
/// ```
impl fmt::Display for Content {
    /// Formats the `Content` enum variant for display.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Content::None => write!(f, ""),
            Content::Ident(idx) => write!(f, "{}", idx),
            Content::Number(val) => write!(f, "{}", val),
            Content::Requirement(req) => write!(f, "{}", req),
            Content::CompareOp(op) => write!(f, "{}", op),
            Content::AssignOp(op) => write!(f, "{}", op),
            Content::ArithmeticOp(op) => write!(f, "{}", op),
            Content::OptimizationOp(op) => write!(f, "{}", op),
        }
    }
}

/// Implements the `DisplayWithInterner` trait for the `Content` enum.
///
/// This implementation enables formatting `Content` values with awareness of
/// interned strings via a `StringInterner`.
///
/// - For the `Ident` variant, which holds an interned string index, it resolves
///   the index to the actual string and formats it with quotes.
/// - For other variants, it defers to the standard `Display` trait implementation.
///
/// This allows proper display of interned identifiers while maintaining fallback
/// for other content types.
///
/// # Example
///
/// ```
/// let ident = Content::Ident(some_idx);
/// let s = ident.to_string_with_interner(&interner);
/// ```
impl InternerDisplay for Content {
    /// Formats the `Content` value using the given formatter and interner.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write output to.
    /// * `interner` - The interner to resolve interned string indices.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &SymbolInterner) -> fmt::Result {
        match self {
            Content::Ident(idx) => {
                // Resolve the interned string or fallback to a placeholder.
                write!(
                    f,
                    "\"{}\"",
                    interner
                        .resolve_symbol(*idx)
                        .unwrap_or(SymbolInterner::UNKNOWN_INTERNED_STRING)
                )
            }
            _ => fmt::Display::fmt(self, f),
        }
    }
}

/// Implements the `PlanningSyntaxDisplay` trait for the `Content` enum.
///
/// This implementation formats the various `Content` variants for syntax display,
/// closely mirroring the behavior of `DisplayWithInterner`.
///
/// - For the `Ident` variant, it resolves the interned string and outputs it enclosed in quotes.
/// - For other variants, it delegates to their standard `Display` formatting.
///
/// This method is intended for rendering `Content` in a source-like syntax format.
///
/// # Example
///
/// ```
/// let content = Content::Ident(idx);
/// content.fmt_planning(&mut formatter, &interner, 0)?;
/// ```
impl SyntaxInternerDisplay for Content {
    /// Formats the `Content` value using the provided formatter and string interner,
    /// applying indentation according to `indent`.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write output to.
    /// * `interner` - The interner used to resolve interned strings.
    /// * `indent` - The indentation level (number of indent units).
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating whether formatting succeeded.
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        match self {
            Content::Ident(idx) => idx.fmt_syntax_with_interner_and_indent(f, interner, indent),
            _ => fmt::Display::fmt(self, f),
        }
    }
}

impl SyntaxContent for Content {
    /// Returns the floating-point literal if this content is a `Float`.
    ///
    /// # Returns
    ///
    /// - `Some(OrderedFloat<f64>)` if the content is a floating-point literal.
    /// - `None` otherwise.
    fn as_number(&self) -> Option<OrderedFloat<f64>> {
        match self {
            Content::Number(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the binary comparison operator if this content is a `BinaryComp`.
    ///
    /// # Returns
    ///
    /// - `Some(BinaryComp)` if the content is a binary comparison operator.
    /// - `None` otherwise.
    fn as_compare_op(&self) -> Option<CompareOp> {
        match self {
            Content::CompareOp(bc) => Some(*bc),
            _ => None,
        }
    }

    /// Returns the assignment operator if this content is an `AssignOp`.
    ///
    /// # Returns
    ///
    /// - `Some(AssignOp)` if the content is an assignment operator.
    /// - `None` otherwise.
    fn as_assign_op(&self) -> Option<AssignOp> {
        match self {
            Content::AssignOp(op) => Some(*op),
            _ => None,
        }
    }

    /// Returns the arithmetic operator if this content is an `ArithmeticOp`.
    ///
    /// # Returns
    ///
    /// - `Some(ArithmeticOp)` if the content is an arithmetic operator.
    /// - `None` otherwise.
    fn as_arithmetic_op(&self) -> Option<ArithmeticOp> {
        match self {
            Content::ArithmeticOp(op) => Some(*op),
            _ => None,
        }
    }

    /// Returns the optimization directive if this content is an `Optimization`.
    ///
    /// # Returns
    ///
    /// - `Some(Optimization)` if the content is an optimization directive.
    /// - `None` otherwise.
    fn as_optimization_op(&self) -> Option<OptimizationOp> {
        match self {
            Content::OptimizationOp(opt) => Some(*opt),
            _ => None,
        }
    }

    /// Returns `true` if the content is `None` (empty).
    ///
    /// # Returns
    ///
    /// - `true` if content is `AstContent::None`.
    /// - `false` otherwise.
    fn is_none(&self) -> bool {
        matches!(self, Content::None)
    }
}

impl RemapSymbol for Content {
    /// Remaps the identifier inside this content if it is an `Ident` using the provided mapping.
    ///
    /// # Parameters
    /// - `map`: A `HashMap` mapping old `Ident`s to their corresponding new `Ident`s.
    ///
    /// # Behavior
    /// - If the content is an `Ident` and a mapping exists in `map`, the identifier is replaced.
    /// - If the content is not an `Ident`, or if no mapping exists for the identifier,
    ///   the content remains unchanged.
    ///
    /// # Notes
    /// - The operation is performed **in place** and is panic-free.
    /// - No error is returned in this implementation; stricter remapping behavior
    ///   can return [`InternerError::MissingIdent`] if desired.
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        if let Content::Ident(id) = self {
            id.remap_idents(map)?;
        }
        Ok(())
    }
}
