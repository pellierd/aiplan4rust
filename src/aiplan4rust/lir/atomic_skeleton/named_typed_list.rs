//! Module defining abstract skeletons shared by predicates and functions in PDDL.
//!
//! This module provides the `NamedTypedList` struct, which encapsulates the common
//! structure of predicates and functions, including their name and parameter signature.
//!
//! These skeletons serve as a base for more specific constructs by factorizing
//! shared fields and behaviors, allowing for code reuse and clearer abstractions.
//!
//! # Overview
//! - `NamedTypedList`: Represents a named typed list, used as a base for predicates and functions.
//!
//! # Example
//! ```
//! use crate::aiplan4rust::lang::Ident;
//! use crate::aiplan4rust::lang::TypedList;
//! use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
//!
//! let pred = NamedTypedList::new(Ident::new("at"), TypedList::new(vec![]));
//! println!("Predicate name: {}", pred.name());
//! ```


use std::fmt;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lang::{StringID, TypedList, TypeID, VariableID};

/// Abstract skeleton common to both predicates and functions in PDDL.
///
/// Encapsulates the shared parts like the name and the signature.
/// This allows factorizing common behavior and fields.
///
/// # Examples
///
/// ```
/// let pred = Skeleton::new(Ident::new("at"), Signature::new(vec![Type::Object]));
/// println!("Name: {}", pred.name());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct NamedTypedList<ID> {
    /// L'identifiant typé (ActionSymbolID, PredicateID, etc.)
    symbol: ID,
    /// La liste des paramètres (Variables et leurs Types)
    parameters: TypedList<VariableID, TypeID>,
}

impl<ID: Copy> NamedTypedList<ID> {
    /// Creates a new `NamedTypedList` with the given name and parameters.
    ///
    /// # Parameters
    ///
    /// - `name`: The identifier representing the name of the predicate or function.
    /// - `parameters`: The list of typed parameters (signature).
    ///
    /// # Returns
    ///
    /// A new instance of `NamedTypedList`.
    pub fn new(symbol: ID, parameters: TypedList<VariableID, TypeID>) -> Self {
        Self { symbol, parameters }
    }
    /// Returns the name of the predicate or function.
    ///
    /// # Returns
    ///
    /// The `Ident` representing the name.
    pub fn symbol(&self) -> ID {
        self.symbol
    }

    /// Sets the symbol identifier (ID) of the predicate, function, or action.
    ///
    /// # Parameters
    ///
    /// - `symbol`: The new typed identifier to set.
    pub fn set_symbol(&mut self, symbol: ID) {
        self.symbol = symbol;
    }

    /// Returns a reference to the parameters (signature).
    ///
    /// # Returns
    ///
    /// A reference to the `TypedList` representing the parameters.
    pub fn parameters(&self) -> &TypedList<VariableID, TypeID> {
        &self.parameters
    }

    /// Returns a mutable reference to the parameters.
    ///
    /// This allows modifying the parameter list directly.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `TypedList`.
    pub fn parameters_mut(&mut self) -> &mut TypedList<VariableID, TypeID> {
        &mut self.parameters
    }

    /// Sets the parameters (signature) of the predicate or function.
    ///
    /// # Parameters
    ///
    /// - `parameters`: The new `TypedList` to set as the parameters.
    pub fn set_parameters(&mut self, parameters: TypedList<VariableID, TypeID>) {
        self.parameters = parameters;
    }

    pub fn arity(&self) -> usize {
        self.parameters.len()
    }
}

impl<ID> fmt::Display for NamedTypedList<ID>
where
    ID: fmt::Display
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[symbol: {}, parameters: {}]", self.symbol, self.parameters)
    }
}
