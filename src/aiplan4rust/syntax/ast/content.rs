//! AST Node Content Representation
//!
//! This module defines [`Content`], an enum used to attach semantic or syntactic meaning
//! to an AST node. Each variant represents a concrete payload, such as an identifier,
//! floating-point value, or PDDL-specific operator (e.g., comparison or assignment).
//!
//! `Content` is a leaf element in the AST: it holds data but no tree structure.
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
//! use [`Content::display_with_context`] with a [`StringInterner`].
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

use std::collections::HashMap;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::elements::{ArithmeticOp, AssignOp, BinaryComp, Optimization, Requirement};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::tree::NodeContent;
use crate::aiplan4rust::serialization::{serialize_ordered_float, deserialize_ordered_float};

use std::fmt;
use std::fmt::Formatter;
use ordered_float::OrderedFloat;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Visitor;
use crate::aiplan4rust::syntax::DisplaySyntax;

/// Represents semantic content associated with an AST node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Content {
    /// No content (default/empty node).
    #[default]
    None,

    /// Interned identifier (references a string in the [`StringInterner`]).
    Ident(Ident),

    /// Floating-point literal (wrapped in [`OrderedFloat`] for total ordering).
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Float(OrderedFloat<f64>),

    /// A requirement flag such as `:typing` or `:equality`.
    Requirement(Requirement),

    /// A comparison operator, e.g. `=`, `<`, `>`.
    BinaryComp(BinaryComp),

    /// An assignment operator, e.g. `assign`, `increase`.
    AssignOp(AssignOp),

    /// An arithmetic operator like `+`, `-`, `*`, `/`.
    ArithmeticOp(ArithmeticOp),

    /// An optimization directive such as `maximize` or `minimize`.
    Optimization(Optimization),
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
    /// Returns `ParserInternalError` if the content is not a `Requirement`.
    pub fn try_requirement(&self) -> Result<Requirement, ParserInternalError> {
        match self {
            Content::Requirement(r) => Ok(*r),
            other => Err(ParserInternalError::new(format!("Expected AstContent::Requirement, found {:?}", other))),
        }
    }
    }

impl fmt::Display for Content {
    /// Formats the `Content` enum for display without any interner resolution.
    ///
    /// Each variant is formatted in a straightforward manner:
    /// - `None` displays as an empty string.
    /// - `Ident` displays using its `Display` impl (usually as `#<value>`).
    /// - Other variants delegate to their respective `Display` impl.
    ///
    /// # Example
    /// ```
    /// let c = Content::Float(3.14);
    /// println!("{}", c); // outputs "3.14"
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Content::None => write!(f, ""),
            Content::Ident(idx) => write!(f, "{}", idx),
            Content::Float(val) => write!(f, "{}", val),
            Content::Requirement(req) => write!(f, "{}", req),
            Content::BinaryComp(comp) => write!(f, "{}", comp),
            Content::AssignOp(assign) => write!(f, "{}", assign),
            Content::ArithmeticOp(op) => write!(f, "{}", op),
            Content::Optimization(opt) => write!(f, "{}", opt),
        }
    }
}

impl DisplayWithInterner for Content {
    /// Formats the `Content` enum using the provided `StringInterner` for
    /// interned string resolution.
    ///
    /// This is important for the `Ident` variant, which stores an index and
    /// needs to be resolved to the actual string via the interner.
    ///
    /// For other variants, it delegates to the regular `Display` implementation.
    ///
    /// # Example
    /// ```
    /// let ident = Content::Ident(some_idx);
    /// let s = ident.to_string_with_interner(&interner);
    /// ```
    fn fmt_with(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        match self {
            Content::Ident(idx) => {
                // Resolve the interned string, or fallback if not found.
                write!(
                    f,
                    "\"{}\"",
                    interner.resolve(*idx).unwrap_or(StringInterner::UNKNOWN_INTERNED_STRING)
                )
            },
            _ => fmt::Display::fmt(self, f),
        }
    }
}

impl DisplaySyntax for Content {
    /// Formats the `Content` enum for syntax display, which is very similar
    /// to `DisplayWithInterner`.
    ///
    /// For `Ident`, it outputs the resolved interned string in quotes.
    /// Other variants use the standard `Display` formatting.
    ///
    /// This method can be used when rendering content for source-like syntax display.
    ///
    /// # Example
    /// ```
    /// let content = Content::Ident(idx);
    /// content.fmt_syntax(&mut formatter, &interner)?;
    /// ```
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        match self {
            Content::Ident(idx) => {
                write!(
                    f,
                    "\"{}\"",
                    interner.resolve(*idx).unwrap_or(StringInterner::UNKNOWN_INTERNED_STRING)
                )
            },
            _ => fmt::Display::fmt(self, f),
        }
    }
}
impl NodeContent for Content {
    /// Returns the identifier if this content is an `Ident`.
    ///
    /// # Returns
    ///
    /// - `Some(Ident)` if the content is an identifier.
    /// - `None` otherwise.
    fn as_ident(&self) -> Option<Ident> {
        match self {
            Content::Ident(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the floating-point literal if this content is a `Float`.
    ///
    /// # Returns
    ///
    /// - `Some(OrderedFloat<f64>)` if the content is a floating-point literal.
    /// - `None` otherwise.
    fn as_float(&self) -> Option<OrderedFloat<f64>> {
        match self {
            Content::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the binary comparison operator if this content is a `BinaryComp`.
    ///
    /// # Returns
    ///
    /// - `Some(BinaryComp)` if the content is a binary comparison operator.
    /// - `None` otherwise.
    fn as_binary_comp(&self) -> Option<BinaryComp> {
        match self {
            Content::BinaryComp(bc) => Some(*bc),
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
    fn as_optimization(&self) -> Option<Optimization> {
        match self {
            Content::Optimization(opt) => Some(*opt),
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

    /// Remaps an identifier in the content if it matches one in the provided mapping.
    ///
    /// This method checks if the content is an `Ident` variant. If it is, and the identifier
    /// exists as a key in the provided map, it replaces the identifier with the corresponding mapped value.
    ///
    /// # Arguments
    ///
    /// * `map` - A reference to a [`HashMap`] that maps old [`Ident`]s to their new replacements.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use crate::aiplan4rust::syntax::elements::Ident;
    ///
    /// let mut content = Content::Ident(Ident::from("old_name"));
    /// let mut map = HashMap::new();
    /// map.insert(Ident::from("old_name"), Ident::from("new_name"));
    ///
    /// content.remap_idents(&map);
    /// assert_eq!(content, Content::Ident(Ident::from("new_name")));
    /// ```
    ///
    /// # Notes
    ///
    /// - If the content is not an `Ident`, this function does nothing.
    /// - The remapping is performed in-place.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        if let Content::Ident(id) = self {
            if let Some(new_id) = map.get(id) {
                *id = *new_id;
            }
        }
    }
}
