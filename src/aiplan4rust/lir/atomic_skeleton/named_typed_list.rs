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

use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::interner::{InternerDisplay, InternerError, StringInterner};
use crate::aiplan4rust::lang::{RemapTypes, StringID, RemapIdents, Type, TypedList};
use crate::aiplan4rust::syntax::SyntaxInternerDisplay;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::syntax;
use crate::aiplan4rust::syntax::lexer::Token;

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
    parameters: TypedList,
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
    pub fn new(name: StringID, parameters: TypedList) -> Self {
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
    pub fn parameters(&self) -> &TypedList {
        &self.parameters
    }

    /// Returns a mutable reference to the parameters.
    ///
    /// This allows modifying the parameter list directly.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `TypedList`.
    pub fn parameters_mut(&mut self) -> &mut TypedList {
        &mut self.parameters
    }

    /// Sets the parameters (signature) of the predicate or function.
    ///
    /// # Parameters
    ///
    /// - `parameters`: The new `TypedList` to set as the parameters.
    pub fn set_parameters(&mut self, parameters: TypedList) {
        self.parameters = parameters;
    }
}

impl RemapIdents for NamedTypedList {
    /// Remaps all identifiers in this `NamedTypedList`, including its main symbol
    /// and its parameters, according to the provided mapping.
    ///
    /// Any `Ident` present in `map` is replaced with the corresponding new value.
    ///
    /// # Parameters
    ///
    /// - `map`: A `HashMap<Ident, Ident>` mapping old identifiers to new identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`InternerError`] if any identifier cannot be remapped according to `map`.
    fn remap_idents(&mut self, map: &HashMap<StringID, StringID>) -> Result<(), InternerError> {
        self.symbol.remap_idents(map)?;
        self.parameters.remap_idents(map)?;
        Ok(())
    }
}

impl RemapTypes for NamedTypedList {
    /// Remaps union types (`Type::Either`) in the parameters according to the provided mapping.
    ///
    /// # Parameters
    /// - `map`: A `HashMap<Type, Ident>` mapping union types to their corresponding primitive `Ident`s.
    ///
    /// # Returns
    /// - `Ok(())` if all parameter types were successfully remapped.
    /// - `Err(LirError)` if an error occurs during remapping.
    fn remap_types(&mut self, map: &HashMap<Type<StringID>, StringID>) -> Result<(), LirError> {
        self.parameters.remap_types(map)?;
        Ok(())
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

impl InternerDisplay for NamedTypedList {
    /// Formats the `NamedTypedList` with a string interner, used for pretty printing.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        write!(f, "[name: ")?;
        self.symbol.fmt_with_interner(f, interner)?;
        write!(f, ", parameters: ")?;
        self.parameters.fmt_with_interner(f, interner)?;
        write!(f, "]")
    }
}

impl SyntaxInternerDisplay for NamedTypedList {
    /// Formats the syntax representation of the `NamedTypedList` using an interner.
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        syntax::display::write_indent(f,indent)?;
        write!(f, "{}", Token::LParen)?;
        self.symbol.fmt_syntax_with_interner_and_indent(f, interner, indent)?;
        write!(f, " ")?;
        self.parameters.fmt_syntax_with_interner_and_indent(f, interner, indent)?;
        write!(f, "{}", Token::RParen)
    }
}
