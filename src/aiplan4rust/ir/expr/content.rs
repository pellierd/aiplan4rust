//! Module `ExprContent`
//!
//! This module defines the [`Content`] enum, which represents the semantic content
//! associated with an AST (Abstract Syntax Tree) node.
//!
//! Each variant of [`Content`] corresponds to a specific type of content that can be
//! attached to a node in the tree, such as an identifier, a floating-point literal,
//! or various language-specific operators.
//!
//! [`Content`] is a leaf element in the AST, meaning it contains data but no child nodes.
//!
//! # Main Variants
//! - `None`: no content (default/empty node).
//! - `Ident`: interned identifier (references a name via [`StringInterner`] for efficient string handling).
//! - `Float`: floating-point literal (wrapped in [`OrderedFloat`] to ensure total ordering).
//! - `BinaryComp`: binary comparison operator (`=`, `<`, `>`, etc.).
//! - `AssignOp`: assignment operator (`assign`, `increase`, etc.).
//! - `ArithmeticOp`: arithmetic operator (`+`, `-`, `*`, `/`).
//! - `Optimization`: optimization directive (`maximize`, `minimize`).
//!
//! # Example Usage
//!
//! ```rust
//! use aiplan4rust::syntax::elements::Ident;
//! use aiplan4rust::interner::StringInterner;
//! use aiplan4rust::syntax::Content;
//! use ordered_float::OrderedFloat;
//!
//! let mut interner = StringInterner::default();
//! let id = interner.intern("load");
//! let content = ExprContent::Ident(id);
//!
//! assert_eq!(content.display_with_context(&interner), "Iden(\"load\")");
//!
//! let float_content = ExprContent::Float(OrderedFloat(3.14));
//! println!("{}", float_content); // Prints: 3.14
//! ```
//!
//! # Display with Context
//!
//! Since identifiers are interned (stored as indices in a table), the method
//! [`Content::display_with_context`] resolves the corresponding string using a
//! [`StringInterner`].
//!
//! This is useful for producing human-readable output, especially for debugging or logging.
//!

use std::collections::HashMap;
use crate::aiplan4rust::syntax::elements::{ArithmeticOp, AssignOp, BinaryComp, Optimization};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::serialization::{serialize_ordered_float, deserialize_ordered_float};
use crate::aiplan4rust::tree::NodeContent;

use std::fmt;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Visitor;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::AstContent;

/// Represents the semantic content attached to an AST node.
///
/// Each variant corresponds to a kind of data that can be attached to the node,
/// such as an identifier, a floating-point literal, or an operator.
///
/// This enum is used to give concrete meaning to syntax nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Content {
    /// No content (empty or default node).
    #[default]
    None,

    /// Interned identifier (references a name in [`StringInterner`]).
    Ident(Ident),

    /// Floating-point literal wrapped in [`OrderedFloat`] for total ordering.
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Float(OrderedFloat<f64>),

    /// Binary comparison operator (`=`, `<`, `>`, etc.).
    BinaryComp(BinaryComp),

    /// Assignment operator (`assign`, `increase`, etc.).
    AssignOp(AssignOp),

    /// Arithmetic operator (`+`, `-`, `*`, `/`).
    ArithmeticOp(ArithmeticOp),

    /// Optimization directive (`maximize`, `minimize`).
    Optimization(Optimization),
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Content::None => write!(f, ""),
            Content::Ident(idx) => write!(f, "Ident({})", idx),
            Content::Float(val) => write!(f, "{}", val),
            Content::BinaryComp(comp) => write!(f, "{:?}", comp),
            Content::AssignOp(assign) => write!(f, "{:?}", assign),
            Content::ArithmeticOp(op) => write!(f, "{:?}", op),
            Content::Optimization(opt) => write!(f, "{:?}", opt),
        }
    }
}

impl DisplayWithInterner for Content {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        match self {
            Content::Ident(idx) => {
                let resolved = interner.resolve(*idx).unwrap_or("(unknown)");
                write!(f, "Iden(\"{}\")", resolved)
            },
            _ => fmt::Display::fmt(self, f),
        }
    }
}

impl NodeContent for Content {
    /// Returns the identifier if the content is an `Ident`.
    fn as_ident(&self) -> Option<Ident> {
        match self {
            Content::Ident(id) => Some(*id),
            _ => None,
        }
    }

    /// Returns the floating-point literal if the content is a `Float`.
    fn as_float(&self) -> Option<OrderedFloat<f64>> {
        match self {
            Content::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the binary comparison operator if the content is a `BinaryComp`.
    fn as_binary_comp(&self) -> Option<BinaryComp> {
        match self {
            Content::BinaryComp(bc) => Some(*bc),
            _ => None,
        }
    }

    /// Returns the assignment operator if the content is an `AssignOp`.
    fn as_assign_op(&self) -> Option<AssignOp> {
        match self {
            Content::AssignOp(op) => Some(*op),
            _ => None,
        }
    }

    /// Returns the arithmetic operator if the content is an `ArithmeticOp`.
    fn as_arithmetic_op(&self) -> Option<ArithmeticOp> {
        match self {
            Content::ArithmeticOp(op) => Some(*op),
            _ => None,
        }
    }

    /// Returns the optimization directive if the content is an `Optimization`.
    fn as_optimization(&self) -> Option<Optimization> {
        match self {
            Content::Optimization(opt) => Some(*opt),
            _ => None,
        }
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

impl TryFrom<&AstContent> for Content {
    type Error = ParserInternalError;

    fn try_from(content: &AstContent) -> Result<Self, Self::Error> {
        match content {
            AstContent::Ident(ident) => Ok(Content::Ident(*ident)),
            AstContent::Float(n) => Ok(Content::Float(*n)),
            AstContent::BinaryComp(op) => Ok(Content::BinaryComp(*op)),
            AstContent::AssignOp(op) => Ok(Content::AssignOp(*op)),
            AstContent::ArithmeticOp(op) => Ok(Content::ArithmeticOp(*op)),
            AstContent::Optimization(op) => Ok(Content::Optimization(*op)),
            AstContent::Requirement(_) => Err(ParserInternalError::new("UnsupportedContent(Requirement".to_string())),
            AstContent::None => Ok(Content::None)
        }
    }
}
