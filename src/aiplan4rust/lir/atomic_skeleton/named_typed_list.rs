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
pub struct NamedTypedList {
    /// Name of the predicate or function.
    symbol: StringID,
    /// Signature describing parameter types and optional return type_checker.
    parameters: TypedList<VariableID, TypeID>,
}

impl NamedTypedList {
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
    pub fn new(name: StringID, parameters: TypedList<VariableID, TypeID>) -> Self {
        Self { symbol: name, parameters }
    }

    /// Returns the name of the predicate or function.
    ///
    /// # Returns
    ///
    /// The `Ident` representing the name.
    pub fn symbol(&self) -> StringID {
        self.symbol
    }

    /// Sets the name of the predicate or function.
    ///
    /// # Parameters
    ///
    /// - `name`: The new name to set.
    pub fn set_name(&mut self, name: StringID) {
        self.symbol = name;
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

impl Display for NamedTypedList {
    /// Formats the `NamedTypedList` as a string for display purposes.
    ///
    /// Output format: `[name: <name>, parameters: <parameters>]`
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[name: ")?;
        self.symbol.fmt(f)?;
        write!(f, ", parameters: ")?;
        self.parameters.fmt(f)?;
        write!(f, "]")
    }
}
