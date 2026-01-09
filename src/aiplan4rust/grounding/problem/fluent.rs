use std::fmt;
use serde::{Deserialize, Serialize};

/// Represents a fluent in a PDDL domain.
///
/// A `Fluent` is a symbolic relation or property that can be true or false in a given state.
/// It is identified by a `symbol` and may have a list of arguments (`arguments`), which are
/// indices pointing to type definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fluent {
    /// Symbolic identifier of the fluent.
    symbol: usize,

    /// Types of the parameters (arguments), represented as indices.
    parameters: Vec<usize>,
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
    pub fn new(symbol: usize, arguments: Vec<usize>) -> Self {
        Self { symbol, parameters: arguments }
    }

    /// Returns the symbolic identifier of the fluent.
    ///
    /// # Returns
    /// The `symbol` of type `usize`.
    pub fn symbol(&self) -> usize {
        self.symbol
    }

    /// Sets the symbolic identifier of the fluent.
    ///
    /// # Parameters
    /// - `symbol`: The new symbolic identifier to assign.
    pub fn set_symbol(&mut self, symbol: usize) {
        self.symbol = symbol;
    }

    /// Returns a reference to the arguments of the fluent.
    ///
    /// # Returns
    /// Reference to a `Vec<usize>` containing the argument type indices.
    pub fn parameters(&self) -> &Vec<usize> {
        &self.parameters
    }

    /// Returns a mutable reference to the arguments of the fluent.
    ///
    /// # Returns
    /// Mutable reference to a `Vec<usize>` containing the argument type indices.
    pub fn parameters_mut(&mut self) -> &mut Vec<usize> {
        &mut self.parameters
    }

    /// Sets the arguments of the fluent.
    ///
    /// # Parameters
    /// - `arguments`: A vector of indices representing the new argument types.
    pub fn set_parameters(&mut self, arguments: Vec<usize>) {
        self.parameters = arguments;
    }
}

impl fmt::Display for Fluent {
    /// Formats the fluent in a PDDL-friendly style: `symbol arg1 arg2 ...`.
    ///
    /// # Parameters
    /// - `f`: The formatter to write to.
    ///
    /// # Returns
    /// A `fmt::Result` indicating success or failure.
    ///
    /// # Example
    /// ```
    /// let f = Fluent::new(1, vec![2, 3]);
    /// println!("{}", f); // Prints: 1 2 3
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let args = self.parameters
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        write!(f, "{} {}", self.symbol, args)
    }
}
