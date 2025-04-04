use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Represents a symbol with associated types.
/// The symbol is of generic type `T`, and the associated types are stored in a vector of the same type.
pub struct TypedSymbol<T>
where
    T: Clone + Display,
{
    symbol: T,
    types: Vec<T>,
}

impl<T> TypedSymbol<T>
where
    T: Clone + Display,
{
    /// Creates a new `TypedSymbol` with the given symbol and associated types.
    ///
    /// # Arguments
    /// * `symbol` - The main symbol of type `T`.
    /// * `types` - A vector of types associated with the symbol.
    ///
    /// # Returns
    /// A `TypedSymbol` instance containing the symbol and its associated types.
    pub fn new(symbol: T, types: Vec<T>) -> Self {
        TypedSymbol { symbol, types }
    }

    /// Returns a reference to the symbol.
    ///
    /// # Returns
    /// A reference to the symbol of type `T`.
    pub fn symbol(&self) -> &T {
        &self.symbol
    }

    /// Returns a reference to the types associated with the symbol.
    ///
    /// # Returns
    /// A reference to a vector containing the associated types.
    pub fn types(&self) -> &Vec<T> {
        &self.types
    }
}

impl<T: fmt::Display> fmt::Display for TypedSymbol<T>
where
    T: Clone,
{
    /// Implements the `Display` trait for `TypedSymbol`.
    /// This will format the symbol and its associated types as a string.
    ///
    /// # Arguments
    /// * `f` - A formatter used to write the formatted string.
    ///
    /// # Returns
    /// A `fmt::Result` indicating whether the formatting succeeded.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Display the main symbol
        write!(f, "{}", self.symbol)?;

        // If there are associated types, display them after the main symbol
        if !self.types.is_empty() {
            write!(f, " - ")?;
            for (i, ty) in self.types.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?; // Add space between types
                }
                write!(f, "{}", ty)?;
            }
        }

        Ok(())
    }
}
