use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::grounding::problem::ids::{ParameterID, PredicateID};

/// Represents a fluent in a PDDL domain.
///
/// A `Fluent` is a symbolic relation or property that can be true or false in a given state.
/// It is identified by a `symbol` and may have a list of arguments (`arguments`), which are
/// indices pointing to type definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fluent {
    /// Symbolic identifier of the fluent.
    symbol: PredicateID,

    /// Types of the parameters (arguments), represented as indices.
    parameters: Vec<ParameterID>,
}

impl Fluent {
    /// Creates a new `Fluent` with the specified `symbol` and `arguments`.
    ///
    /// # Parameters
    /// - `symbol`: Symbolic identifier for the fluent.
    /// - `arguments`: Vector of indices representing the types of each argument.
    ///
    /// # Returns
    /// A new instance of `Fluent`.
    ///
    /// # Example
    /// ```
    /// let f = Fluent::new(1, vec![2, 3]);
    /// ```
    pub fn new(symbol: PredicateID, arguments: Vec<ParameterID>) -> Self {
        Self { symbol, parameters: arguments }
    }

    /// Returns the symbolic identifier of the fluent.
    ///
    /// # Returns
    /// The `symbol` of type `usize`.
    pub fn symbol(&self) -> PredicateID {
        self.symbol
    }

    /// Sets the symbolic identifier of the fluent.
    ///
    /// # Parameters
    /// - `symbol`: The new symbolic identifier to assign.
    pub fn set_symbol(&mut self, symbol: PredicateID) {
        self.symbol = symbol;
    }

    /// Returns a reference to the arguments of the fluent.
    ///
    /// # Returns
    /// Reference to a `Vec<usize>` containing the argument type indices.
    pub fn parameters(&self) -> &Vec<ParameterID> {
        &self.parameters
    }

    /// Returns a mutable reference to the arguments of the fluent.
    ///
    /// # Returns
    /// Mutable reference to a `Vec<usize>` containing the argument type indices.
    pub fn parameters_mut(&mut self) -> &mut Vec<ParameterID> {
        &mut self.parameters
    }

    /// Sets the arguments of the fluent.
    ///
    /// # Parameters
    /// - `arguments`: A vector of indices representing the new argument types.
    pub fn set_parameters(&mut self, arguments: Vec<ParameterID>) {
        self.parameters = arguments;
    }
}

impl fmt::Display for Fluent {
    /// Formats the fluent in a PDDL-friendly style: `symbol arg1 arg2 ...`.
    ///
    /// # Example
    /// ```
    /// let f = Fluent {
    ///     symbol: PredicateID(1),
    ///     parameters: vec![
    ///         ParameterID::Object(ObjectID(3)),
    ///         ParameterID::ObjectFluent(ObjectFluentID(5))
    ///     ],
    /// };
    /// println!("{}", f); // Prints: Predicate#1 Object#3 ObjectFluent#5
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Convertit tous les paramètres en chaînes via leur Display
        let args = self.parameters
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        if args.is_empty() {
            write!(f, "({})", self.symbol)
        } else {
            write!(f, "({} {})", self.symbol, args)
        }
    }
}
