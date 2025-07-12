//! Predicate Signature Representation (`AtomicFormulaSkeleton`)
//!
//! This module defines the [`Formula`] struct, which represents the signature of a PDDL predicate.
//! Predicates have a name and typed parameters, and always return a boolean value (implicitly).
//!
//! This structure is re-exported as [`AtomicFormulaSkeleton`] from the parent module.
//!
//! # Example Use
//!
//! ```rust
//! use aiplan4rust::lir::atomic_skeleton::AtomicFormulaSkeleton;
//! use aiplan4rust::lang::{Ident, TypedList};
//!
//! let pred = AtomicFormulaSkeleton::new(Ident::new("at"), TypedList::empty());
//! ```

use std::fmt;
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{Ident, TypedList};
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::syntax::SyntaxDisplay;
use crate::aiplan4rust::arena::Arena;

/// Represents the signature of an atomic formula (predicate) in a PDDL-like domain.
///
/// A `Formula` stores:
/// - The identifier (name) of the predicate.
/// - The list of typed parameters (its arguments).
///
/// Unlike functions, predicates always return a Boolean value (implicitly true or false),
/// so their return type is always `None`.
///
/// This type internally uses [`NamedTypedList`] to factor out the shared representation
/// of the identifier and its parameters.
///
/// # Examples
///
/// ```
/// use aiplan4rust::lang::{Ident, Type, TypedList};
/// use aiplan4rust::lir::atomic_skeleton::formula::Formula;
///
/// let formula = Formula::new(
///     Ident::new("at"),
///     TypedList::from(vec![Type::Object, Type::Location]),
/// );
///
/// assert_eq!(formula.signature().arity(), 2);
/// assert_eq!(formula.signature().return_type(), None);
/// ```
///
/// # Implementation Notes
///
/// - Implements [`Deref`] and [`DerefMut`] to access the underlying [`NamedTypedList`] transparently.
/// - Supports pretty-printing with or without an interner (see [`InternerDisplay`] and [`SyntaxDisplay`]).
/// - Can be constructed directly or parsed from an AST node via [`FromAst`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Formula {
    /// Underlying skeleton holding the identifier and parameters.
    header: NamedTypedList,
}

impl Formula {
    /// Creates a new `Formula` (predicate signature) from a name and parameter list.
    ///
    /// # Arguments
    ///
    /// * `name` - The identifier of the predicate.
    /// * `parameters` - The list of typed parameters.
    ///
    /// # Returns
    ///
    /// A `Formula` instance whose return type is always `None`.
    pub fn new(name: Ident, parameters: TypedList) -> Self {
        let header = NamedTypedList::new(name, parameters);
        Self { header }
    }
}

// Allow direct access to NamedTypedList methods.
impl Deref for Formula {
    type Target = NamedTypedList;

    fn deref(&self) -> &Self::Target {
        &self.header
    }
}

impl DerefMut for Formula {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.header
    }
}

impl FromAst for Formula {
    /// Parses a `Formula` from an AST node.
    ///
    /// The expected AST node structure:
    /// - Child 0: The identifier.
    /// - Child 1: The typed parameter list.
    ///
    /// # Errors
    ///
    /// Returns a [`ParserInternalError`] if the node does not have the expected structure.
    fn from_ast(
        node: &AstNode,
        ast: &Arena<AstNode>,
    ) -> Result<Self, ParserInternalError> {
        let header = NamedTypedList::from_ast(node, ast)?;
        Ok(Formula { header })
    }
}

impl fmt::Display for Formula {
    /// Formats the formula in a human-readable form.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.header.fmt(f)
    }
}

impl InternerDisplay for Formula {
    /// Formats the formula using the provided interner to resolve identifiers.
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.header.fmt_with_interner(f, interner)
    }
}

impl SyntaxDisplay for Formula {
    /// Formats the formula in a syntax-oriented form (e.g., PDDL representation).
    fn fmt_syntax_with_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        self.header.fmt_syntax_with_indent(f, interner, indent)
    }
}
