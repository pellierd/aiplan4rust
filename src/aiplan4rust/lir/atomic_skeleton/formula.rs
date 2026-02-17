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

use crate::aiplan4rust::lang::{PredicateID, TypeID, TypedList, VariableID};
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::lir::symbol_registry::SymbolRegistry;

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
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Formula {
    /// Underlying skeleton holding the identifier and parameters.
    header: NamedTypedList<PredicateID>,
    variable_symbols: SymbolRegistry<VariableID>,
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
    pub fn new(predicate: PredicateID, parameters: TypedList<VariableID, TypeID>) -> Self {
        let header = NamedTypedList::new(predicate, parameters);
        Self {
            header,
            variable_symbols: SymbolRegistry::new()
        }
    }

    /// Permet d'ajouter les symboles après la création de manière élégante.
    /// Usage : Action::new_simple(...).with_symbols(ma_table)
    pub fn with_variable_symbols(mut self, symbols: SymbolRegistry<VariableID>) -> Self {
        self.variable_symbols = symbols;
        self
    }

    pub fn predicate_id(&self) -> PredicateID {
        self.header.symbol()
    }

    /// Accès en lecture seule à la table des noms (symboles) des variables.
    /// À utiliser pour le rendu ou les messages d'erreur.
    pub fn variable_symbols(&self) -> &SymbolRegistry<VariableID> { &self.variable_symbols }

    /// Accès mutable à la table des noms des variables.
    pub fn variable_symbols_mut(&mut self) -> &mut SymbolRegistry<VariableID> { &mut self.variable_symbols }

}

// Allow direct access to NamedTypedList methods.
impl Deref for Formula {
    type Target = NamedTypedList<PredicateID>;

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
