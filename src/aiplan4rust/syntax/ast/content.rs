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
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::elements::{ArithmeticOp, AssignOp, BinaryComp, Ident, Optimization, Requirement};
use crate::aiplan4rust::syntax::StringInterner;

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
    /// Returns the identifier if this content is an `Ident`.
    ///
    /// # Returns
    ///
    /// - `Some(Ident)` if the content is an identifier.
    /// - `None` otherwise.
    pub fn as_ident(&self) -> Option<Ident> {
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
    pub fn as_float(&self) -> Option<OrderedFloat<f64>> {
        match self {
            Content::Float(f) => Some(*f),
            _ => None,
        }
    }

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

    /// Returns the binary comparison operator if this content is a `BinaryComp`.
    ///
    /// # Returns
    ///
    /// - `Some(BinaryComp)` if the content is a binary comparison operator.
    /// - `None` otherwise.
    pub fn as_binary_comp(&self) -> Option<BinaryComp> {
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
    pub fn as_assign_op(&self) -> Option<AssignOp> {
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
    pub fn as_arithmetic_op(&self) -> Option<ArithmeticOp> {
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
    pub fn as_optimization(&self) -> Option<Optimization> {
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
    pub fn is_none(&self) -> bool {
        matches!(self, Content::None)
    }

    /// Returns the identifier if this content is an `Ident`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not an `Ident`.
    pub fn try_ident(&self) -> Result<Ident, ParserInternalError> {
        match self {
            Content::Ident(id) => Ok(*id),
            other => Err(ParserInternalError::new(format!("Expected AstContent::Ident, found {:?}", other))),
        }
    }

    /// Returns the floating-point literal if this content is a `Float`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not a `Float`.
    pub fn try_float(&self) -> Result<OrderedFloat<f64>, ParserInternalError> {
        match self {
            Content::Float(f) => Ok(*f),
            other => Err(ParserInternalError::new(format!("Expected AstContent::Float, found {:?}", other))),
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

    /// Returns the binary comparison operator if this content is a `BinaryComp`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not a `BinaryComp`.
    pub fn try_binary_comp(&self) -> Result<BinaryComp, ParserInternalError> {
        match self {
            Content::BinaryComp(bc) => Ok(*bc),
            other => Err(ParserInternalError::new(format!("Expected AstContent::BinaryComp, found {:?}", other))),
        }
    }

    /// Returns the assignment operator if this content is an `AssignOp`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not an `AssignOp`.
    pub fn try_assign_op(&self) -> Result<AssignOp, ParserInternalError> {
        match self {
            Content::AssignOp(op) => Ok(*op),
            other => Err(ParserInternalError::new(format!("Expected AstContent::AssignOp, found {:?}", other))),
        }
    }

    /// Returns the arithmetic operator if this content is an `ArithmeticOp`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not an `ArithmeticOp`.
    pub fn try_arithmetic_op(&self) -> Result<ArithmeticOp, ParserInternalError> {
        match self {
            Content::ArithmeticOp(op) => Ok(*op),
            other => Err(ParserInternalError::new(format!("Expected AstContent::ArithmeticOp, found {:?}", other))),
        }
    }

    /// Returns the optimization directive if this content is an `Optimization`.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the content is not an `Optimization`.
    pub fn try_optimization(&self) -> Result<Optimization, ParserInternalError> {
        match self {
            Content::Optimization(opt) => Ok(*opt),
            other => Err(ParserInternalError::new(format!("Expected AstContent::Optimization, found {:?}", other))),
        }
    }

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
            Content::None => "None".to_string(),
            Content::Ident(idx) => format!("Iden(\"{}\")", ctx.get_str(*idx).unwrap_or("(unknown)").to_string()),
            Content::Float(val) => format!("Float({})", val),
            Content::Requirement(req) => format!("Requirement({})", req),
            Content::BinaryComp(comp) => format!("BinaryComp({})", comp),
            Content::AssignOp(assign) => format!("AssignOp({})", assign),
            Content::ArithmeticOp(op) => format!("ArithmeticOp({})", op),
            Content::Optimization(opt) => format!("Optimization({})", opt),
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
            Content::BinaryComp(comp) => write!(f, "{:?}", comp),
            Content::AssignOp(assign) => write!(f, "{:?}", assign),
            Content::ArithmeticOp(op) => write!(f, "{:?}", op),
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
