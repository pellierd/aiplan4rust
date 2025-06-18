//! AST Node Content Representation
//!
//! This module defines [`Content`], an enum used to attach semantic or syntactic meaning
//! to an AST node. Each variant represents a concrete payload, such as an identifier,
//! floating-point value, or PDDL-specific operator (e.g., comparison or assignment).
//!
//! `Content` is a leaf element in the AST: it holds data but no tree structure.
//!
//! It also includes custom (de)serialization for floating-point values using
//! [`OrderedFloat<f64>`], to ensure proper `Eq`/`Ord` semantics and deterministic serialization.
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

use std::fmt;
use ordered_float::OrderedFloat;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Visitor;

use crate::aiplan4rust::syntax::elements::{
    ArithmeticOp, AssignOp, BinaryComp, Optimization, Requirement,
};
use crate::aiplan4rust::syntax::StringInterner;

/// Represents semantic content associated with an AST node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Content {
    /// No content (default/empty node).
    #[default]
    None,

    /// Interned identifier (references a string in the [`StringInterner`]).
    Ident(usize),

    /// Floating-point literal (wrapped in [`OrderedFloat`] for total ordering).
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Float(OrderedFloat<f64>),

    /// A requirement flag such as `:typing` or `:equality`.
    Requirement(Requirement),

    /// A comparison operator, e.g. `=`, `<`, `>`.
    Comparison(BinaryComp),

    /// An assignment operator, e.g. `assign`, `increase`.
    Assign(AssignOp),

    /// An arithmetic operator like `+`, `-`, `*`, `/`.
    Operation(ArithmeticOp),

    /// An optimization directive such as `maximize` or `minimize`.
    Optimization(Optimization),
}

impl Content {
    /// Returns a string representation of this `Content` using a [`StringInterner`].
    ///
    /// This is especially useful for resolving interned identifiers to readable names.
    ///
    /// # Arguments
    /// * `ctx` — A reference to the string interner used during parsing.
    ///
    /// # Example
    /// ```
    /// let mut interner = StringInterner::default();
    /// let id = interner.intern("load");
    /// let c = Content::Ident(id);
    /// assert_eq!(c.display_with_context(&interner), "load");
    /// ```
    pub fn display_with_context(&self, ctx: &StringInterner) -> String {
        match self {
            Content::None => "".to_string(),
            Content::Ident(idx) => ctx.get_str(*idx).unwrap_or("(unknown)").to_string(),
            Content::Float(val) => format!("{}", val),
            Content::Requirement(req) => format!("{:?}", req),
            Content::Comparison(comp) => format!("{:?}", comp),
            Content::Assign(assign) => format!("{:?}", assign),
            Content::Operation(op) => format!("{:?}", op),
            Content::Optimization(opt) => format!("{:?}", opt),
        }
    }
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Content::None => write!(f, ""),
            Content::Ident(idx) => write!(f, "Ident({})", idx),
            Content::Float(val) => write!(f, "{}", val),
            Content::Requirement(req) => write!(f, "{:?}", req),
            Content::Comparison(comp) => write!(f, "{:?}", comp),
            Content::Assign(assign) => write!(f, "{:?}", assign),
            Content::Operation(op) => write!(f, "{:?}", op),
            Content::Optimization(opt) => write!(f, "{:?}", opt),
        }
    }
}

/// Custom serialization for `OrderedFloat<f64>`.
///
/// Ensures `OrderedFloat` can be serialized as a normal `f64`.
fn serialize_ordered_float<S>(x: &OrderedFloat<f64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(x.into_inner())
}

/// Custom deserialization for `OrderedFloat<f64>`.
///
/// Ensures that a `f64` is deserialized into an `OrderedFloat`, preserving ordering behavior.
fn deserialize_ordered_float<'de, D>(deserializer: D) -> Result<OrderedFloat<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OrderedFloatVisitor;

    impl<'de> Visitor<'de> for OrderedFloatVisitor {
        type Value = OrderedFloat<f64>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a floating point number")
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(OrderedFloat(value))
        }
    }

    deserializer.deserialize_f64(OrderedFloatVisitor)
}
