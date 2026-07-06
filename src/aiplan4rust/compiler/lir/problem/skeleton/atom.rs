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

use core::fmt::Debug;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};

use crate::aiplan4rust::compiler::lir::problem::skeleton::NamedTypedList;
use crate::aiplan4rust::compiler::lir::problem::SymbolRegistry;
use crate::aiplan4rust::compiler::lir::renderers;
use crate::aiplan4rust::compiler::lir::renderers::{
    LiftedDebugDisplay, LiftedSyntaxDisplay, LirRenderContext,
};
use crate::aiplan4rust::support::lang::{PredicateSymbolId, TypedListId, VariableId};

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
    header: NamedTypedList<PredicateSymbolId>,
    variable_symbols: SymbolRegistry<VariableId>,
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
    pub fn new(predicate: PredicateSymbolId, parameters: TypedListId) -> Self {
        let header = NamedTypedList::new(predicate, parameters);
        Self {
            header,
            variable_symbols: SymbolRegistry::new(),
        }
    }

    /// Permet d'ajouter les symboles après la création de manière élégante.
    /// Usage : Action::new_simple(...).with_symbols(ma_table)
    pub fn with_variable_symbols(mut self, symbols: SymbolRegistry<VariableId>) -> Self {
        self.variable_symbols = symbols;
        self
    }

    pub fn predicate_id(&self) -> PredicateSymbolId {
        self.header.symbol()
    }

    /// Accès en lecture seule à la table des noms (symboles) des variables.
    /// À utiliser pour le rendu ou les messages d'erreur.
    pub fn variable_symbols(&self) -> &SymbolRegistry<VariableId> {
        &self.variable_symbols
    }

    /// Accès mutable à la table des noms des variables.
    pub fn variable_symbols_mut(&mut self) -> &mut SymbolRegistry<VariableId> {
        &mut self.variable_symbols
    }
    #[inline]
    pub fn set_parameters(&mut self, parameters: TypedListId) {
        self.header.set_parameters(parameters);
    }
}

// Allow direct access to NamedTypedList methods.
impl Deref for Formula {
    type Target = NamedTypedList<PredicateSymbolId>;

    fn deref(&self) -> &Self::Target {
        &self.header
    }
}

impl DerefMut for Formula {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.header
    }
}

impl LiftedSyntaxDisplay for Formula {
    /// Rendu syntaxique PDDL : (at ?obj - object ?loc - location)
    fn fmt_syntax(&self, f: &mut fmt::Formatter<'_>, ctx: &LirRenderContext) -> fmt::Result {
        let local_ctx = ctx.with_variables(&self.variable_symbols);
        renderers::syntax::atom::render(f, self, &local_ctx)
    }
}

impl LiftedDebugDisplay for Formula {
    /// Rendu technique : PredicateSkeleton{ (at [p#2] ?obj [v#0] - object [t#1]) }
    fn fmt_debug(&self, f: &mut fmt::Formatter<'_>, ctx: &LirRenderContext) -> fmt::Result {
        let local_ctx = ctx.with_variables(&self.variable_symbols);
        renderers::debug::atom::render(f, self, &local_ctx)
    }
}
