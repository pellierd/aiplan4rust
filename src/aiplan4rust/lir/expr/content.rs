//! Module `ExprContent`
//!
//! This module defines the [`Content`] enum, which represents the semantic content
//! associated with an AST (Abstract Syntax Tree) syntax node.
//!
//! Each variant of [`Content`] corresponds to a specific type_checker of content that can be
//! attached to a syntax node in the arena, such as an identifier, a floating-point literal,
//! or various language-specific operators.
//!
//! [`Content`] serves as a leaf element in the AST: it contains concrete data but does not
//! have child nodes.
//!
//! # Main Variants
//!
//! - `None`: Represents no content (default/empty syntax).
//! - `Ident`: An interned identifier (references a name via [`StringInterner`] for efficient string storage).
//! - `Float`: A floating-point literal (wrapped in [`OrderedFloat`] to guarantee total ordering).
//! - `BinaryComp`: A binary comparison operator (`=`, `<`, `>`, etc.).
//! - `AssignOp`: An assignment operator (`assign`, `increase`, etc.).
//! - `ArithmeticOp`: An arithmetic operator (`+`, `-`, `*`, `/`).
//! - `Optimization`: An optimization directive (`maximize`, `minimize`).
//!
//! # Display and Debugging
//!
//! Identifiers are stored as interned indices, so the method [`Content::display_with_context`]
//! resolves these to human-readable strings using a [`StringInterner`]. This is useful
//! for pretty-printing, debugging, and logging.
//!
//! # Example
//!
//! ```rust
//! use aiplan4rust::syntax::elements::Ident;
//! use aiplan4rust::interner::StringInterner;
//! use aiplan4rust::syntax::Content;
//! use ordered_float::OrderedFloat;
//!
//! let mut interner = StringInterner::default();
//! let id = interner.intern("load");
//! let content = Content::Ident(id);
//!
//! assert_eq!(content.display_with_context(&interner), "Iden(\"load\")");
//!
//! let float_content = Content::Float(OrderedFloat(3.14));
//! println!("{}", float_content); // Prints: 3.14
//! ```
//!
//! # Identifier Remapping
//!
//! The [`Content::remap_idents`] method allows in-place remapping of interned identifiers
//! according to a provided mapping. This is useful during transformations or renaming phases.

use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, Ident, Optimization};
use crate::aiplan4rust::serialization::{deserialize_ordered_float, serialize_ordered_float};
use crate::aiplan4rust::syntax::ast::AstContent;
use crate::aiplan4rust::syntax::tree::SyntaxContent;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use crate::aiplan4rust::lir::expr::error::ExprError;

/// Represents the semantic content attached to an AST syntax node.
///
/// Each variant corresponds to a kind of concrete data associated with the syntax,
/// such as identifiers, literals, or operators.
///
/// This enum is a leaf in the syntax tree — it contains data but no child nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Content {
    /// No content (empty/default syntax node).
    #[default]
    None,

    /// Interned identifier referencing a name stored in a [`StringInterner`].
    Ident(Ident),

    /// Floating-point literal wrapped in [`OrderedFloat`] to ensure total ordering.
    #[serde(
        serialize_with = "serialize_ordered_float",
        deserialize_with = "deserialize_ordered_float"
    )]
    Float(OrderedFloat<f64>),

    /// Binary comparison operator (e.g. `=`, `<`, `>`, etc.).
    BinaryComp(BinaryComp),

    /// Assignment operator (e.g. `assign`, `increase`, etc.).
    AssignOp(AssignOp),

    /// Arithmetic operator (e.g. `+`, `-`, `*`, `/`).
    ArithmeticOp(ArithmeticOp),

    /// Optimization directive (e.g. `maximize`, `minimize`).
    Optimization(Optimization),
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Content::None => write!(f, ""),
            Content::Ident(idx) => write!(f, "Ident({})", idx),
            Content::Float(val) => write!(f, "{}", val),
            Content::BinaryComp(comp) => write!(f, "{}", comp),
            Content::AssignOp(assign) => write!(f, "{}", assign),
            Content::ArithmeticOp(op) => write!(f, "{}", op),
            Content::Optimization(opt) => write!(f, "{}", opt),
        }
    }
}

impl InternerDisplay for Content {
    /// Displays the content with context from a [`StringInterner`], resolving identifiers to strings.
    ///
    /// For non-identifier variants, falls back to the default [`Display`] implementation.
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        match self {
            Content::Ident(idx) => {
                let resolved = interner.resolve_ident(*idx).unwrap_or("(unknown)");
                write!(f, "Iden(\"{}\")", resolved)
            }
            _ => fmt::Display::fmt(self, f),
        }
    }
}

impl SyntaxContent for Content {
    fn as_ident(&self) -> Option<Ident> {
        match self {
            Content::Ident(id) => Some(*id),
            _ => None,
        }
    }

    fn as_float(&self) -> Option<OrderedFloat<f64>> {
        match self {
            Content::Float(f) => Some(*f),
            _ => None,
        }
    }

    fn as_binary_comp(&self) -> Option<BinaryComp> {
        match self {
            Content::BinaryComp(bc) => Some(*bc),
            _ => None,
        }
    }

    fn as_assign_op(&self) -> Option<AssignOp> {
        match self {
            Content::AssignOp(op) => Some(*op),
            _ => None,
        }
    }

    fn as_arithmetic_op(&self) -> Option<ArithmeticOp> {
        match self {
            Content::ArithmeticOp(op) => Some(*op),
            _ => None,
        }
    }

    fn as_optimization(&self) -> Option<Optimization> {
        match self {
            Content::Optimization(opt) => Some(*opt),
            _ => None,
        }
    }

    /// Remaps interned identifiers in place according to a provided map.
    ///
    /// If the content is an `Ident` and its current identifier is found in `map`,
    /// it will be replaced by the mapped identifier.
    ///
    /// # Arguments
    ///
    /// * `map` - A map from old `Ident` values to their replacements.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use aiplan4rust::syntax::elements::Ident;
    ///
    /// let mut content = Content::Ident(Ident::from("old_name"));
    /// let mut map = HashMap::new();
    /// map.insert(Ident::from("old_name"), Ident::from("new_name"));
    ///
    /// content.remap_idents(&map);
    /// assert_eq!(content, Content::Ident(Ident::from("new_name")));
    /// ```
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        if let Content::Ident(id) = self {
            if let Some(new_id) = map.get(id) {
                *id = *new_id;
            }
        }
    }
}

impl TryFrom<&AstContent> for Content {
    type Error = ExprError;

    /// Attempts to convert an [`AstContent`] reference into a [`Content`].
    ///
    /// Returns an error if the content is unsupported in the `expr` module.
    fn try_from(content: &AstContent) -> Result<Self, Self::Error> {
        match content {
            AstContent::Ident(ident) => Ok(Content::Ident(*ident)),
            AstContent::Float(n) => Ok(Content::Float(*n)),
            AstContent::BinaryComp(op) => Ok(Content::BinaryComp(*op)),
            AstContent::AssignOp(op) => Ok(Content::AssignOp(*op)),
            AstContent::ArithmeticOp(op) => Ok(Content::ArithmeticOp(*op)),
            AstContent::Optimization(op) => Ok(Content::Optimization(*op)),
            AstContent::Requirement(req) => {
                Err(ExprError::unsupported_content(AstContent::Requirement(*req)))
            }
            AstContent::None => Ok(Content::None),
        }
    }
}
