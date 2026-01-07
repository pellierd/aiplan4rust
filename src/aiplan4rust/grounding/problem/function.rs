use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a function object in a PDDL domain.
///
/// A `Function` is a symbolic relation that maps a set of arguments to a return type (`ty`).
/// It is identified by a `symbol` and may have multiple arguments, represented as indices
/// pointing to type definitions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Function {
    /// Symbolic name of the function (e.g., "location-of").
    symbol: usize,

    /// Types of the function parameters (arguments), represented as indices.
    arguments: Vec<usize>,

    /// Return type of the function, represented as an index.
    ty: usize,
}

impl Function {
    /// Creates a new `Function` with the specified `symbol`, `arguments`, and return type `ty`.
    ///
    /// # Parameters
    /// - `symbol`: Symbolic identifier for the function.
    /// - `arguments`: Vector of indices representing the types of each argument.
    /// - `ty`: Index representing the return type of the function.
    ///
    /// # Returns
    /// A new instance of `Function`.
    ///
    /// # Example
    /// ```
    /// let f = Function::new(1, vec![2, 3], 5);
    /// ```
    pub fn new(symbol: usize, arguments: Vec<usize>, ty: usize) -> Self {
        Self { symbol, arguments, ty }
    }

    // ----- Getters -----

    /// Returns the symbolic identifier of the function.
    ///
    /// # Returns
    /// The `symbol` of type `usize`.
    pub fn symbol(&self) -> usize {
        self.symbol
    }

    /// Returns a reference to the arguments of the function.
    ///
    /// # Returns
    /// Reference to a `Vec<usize>` containing the argument type indices.
    pub fn arguments(&self) -> &Vec<usize> {
        &self.arguments
    }

    /// Returns a mutable reference to the arguments of the function.
    ///
    /// # Returns
    /// Mutable reference to a `Vec<usize>` containing the argument type indices.
    pub fn arguments_mut(&mut self) -> &mut Vec<usize> {
        &mut self.arguments
    }

    /// Returns the return type index of the function.
    ///
    /// # Returns
    /// `ty` of type `usize`.
    pub fn ty(&self) -> usize {
        self.ty
    }

    // ----- Setters -----

    /// Sets the symbolic identifier of the function.
    ///
    /// # Parameters
    /// - `symbol`: New symbolic identifier.
    pub fn set_symbol(&mut self, symbol: usize) {
        self.symbol = symbol;
    }

    /// Sets the argument types of the function.
    ///
    /// # Parameters
    /// - `arguments`: Vector of indices representing the new argument types.
    pub fn set_arguments(&mut self, arguments: Vec<usize>) {
        self.arguments = arguments;
    }

    /// Sets the return type of the function.
    ///
    /// # Parameters
    /// - `ty`: New return type index.
    pub fn set_ty(&mut self, ty: usize) {
        self.ty = ty;
    }
}

impl fmt::Display for Function {
    /// Formats the function in a PDDL-friendly style:
    /// `symbol arg1 arg2 ... - ty` if there are arguments,
    /// or `symbol - ty` if there are no arguments.
    ///
    /// # Parameters
    /// - `f`: The formatter to write to.
    ///
    /// # Returns
    /// `fmt::Result` indicating success or failure.
    ///
    /// # Example
    /// ```
    /// let f = Function::new(1, vec![2, 3], 5);
    /// println!("{}", f); // Prints: 1 2 3 - 5
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let args = if !self.arguments.is_empty() {
            self.arguments
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            String::new()
        };

        if args.is_empty() {
            write!(f, "{} - {}", self.symbol, self.ty)
        } else {
            write!(f, "{} {} - {}", self.symbol, args, self.ty)
        }
    }
}
