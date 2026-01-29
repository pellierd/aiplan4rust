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

use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{StringID, TypedList};
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::semantic::symbol::{Symbol, SymbolKind};
use crate::aiplan4rust::syntax::SyntaxInternerDisplay;

/// Represents the signature of an atomic formula (predicate) in a PDDL-like domain.
///
/// A `Formula` stores:
/// - The identifier (name) of the predicate.
/// - The list of typed parameters (its arguments).
///
/// Unlike functions, predicates always return a Boolean value (implicitly true or false),
/// so their return type_checker is always `None`.
///
/// This type_checker internally uses [`NamedTypedList`] to factor out the shared representation
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
/// - Supports pretty-printing with or without an interner (see [`InternerDisplay`] and [`SyntaxInternerDisplay`]).
/// - Can be constructed directly or parsed from an AST syntax via [`FromAst`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
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
    /// A `Formula` instance whose return type_checker is always `None`.
    pub fn new(name: StringID, parameters: TypedList<StringID>) -> Self {
        let header = NamedTypedList::new(name, parameters);
        Self { header }
    }

    /// Creates a new `Formula` from an already constructed header.
    ///
    /// This constructor is intended for **internal use only** within the crate,
    /// specifically for the LIR encoding process where a [`NamedTypedList`]
    /// has already been parsed from the AST.
    ///
    /// # Arguments
    ///
    /// * `header` - A fully constructed named typed list representing the predicate and its terms.
    ///
    /// # Returns
    ///
    /// A new `Formula` instance taking ownership of the provided header.
    pub(crate) fn from_header(header: NamedTypedList) -> Self {
        Self { header }
    }

    pub fn predicate(&self) -> Symbol {
        Symbol::new( self.header.symbol(), SymbolKind::Predicate)
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

impl SyntaxInternerDisplay for Formula {
    /// Formats the formula in a syntax-oriented form (e.g., PDDL representation).
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        self.header.fmt_syntax_with_interner_and_indent(f, interner, indent)
    }
}
