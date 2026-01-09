use serde::{Deserialize, Serialize};
use std::fmt;
use crate::aiplan4rust::grounding::problem::Type;

/// Represents a function or object in a grounded PDDL problem.
///
/// - `parameters` are the argument types (indices into the type table).
/// - `ty` is the return type (index into the type table).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Function {
    /// Symbol index in the symbol table.
    symbol: usize,

    /// Argument type indices (indices into the type table).
    parameters: Vec<usize>,

    /// Return type index (into the type table).
    ty: Type,
}

impl Function {
    /// Creates a new function.
    ///
    /// # Parameters
    /// - `symbol`: function symbol index
    /// - `arguments`: argument type indices
    /// - `ty`: return type
    ///
    /// # Returns
    /// A new `Function` instance
    pub fn new(symbol: usize, arguments: Vec<usize>, ty: Type) -> Self {
        Self { symbol, parameters: arguments, ty }
    }

    /// Creates a new object (function with no arguments).
    ///
    /// # Parameters
    /// - `symbol`: object symbol index
    /// - `ty`: object type
    ///
    /// # Returns
    /// A `Function` representing an object
    pub fn object(symbol: usize, ty: Type) -> Self {
        Self::new(symbol, Vec::new(), ty)
    }

    /// Returns the symbol index.
    ///
    /// # Returns
    /// Symbol index (`usize`)
    pub fn symbol(&self) -> usize {
        self.symbol
    }

    /// Returns a reference to the argument types.
    ///
    /// # Returns
    /// Reference to `Vec<usize>` containing argument type indices
    pub fn parameters(&self) -> &Vec<usize> {
        &self.parameters
    }

    /// Returns a mutable reference to the argument types.
    ///
    /// # Returns
    /// Mutable reference to `Vec<usize>` containing argument type indices
    pub fn parameters_mut(&mut self) -> &mut Vec<usize> {
        &mut self.parameters
    }

    /// Returns the return type index.
    ///
    /// # Returns
    /// Return type
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    /// Sets the symbol index.
    ///
    /// # Parameters
    /// - `symbol`: new symbol index
    pub fn set_symbol(&mut self, symbol: usize) {
        self.symbol = symbol;
    }

    /// Sets the argument types.
    ///
    /// # Parameters
    /// - `arguments`: new argument type indices
    pub fn set_parameters(&mut self, arguments: Vec<usize>) {
        self.parameters = arguments;
    }

    /// Sets the return type index.
    ///
    /// # Parameters
    /// - `ty`: new return type
    pub fn set_ty(&mut self, ty: Type) {
        self.ty = ty;
    }
}

impl fmt::Display for Function {
    /// Formats the function in PDDL-like style.
    ///
    /// `symbol arg1 arg2 ... - ty` if arguments exist, or `symbol - ty` if none.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol)?;
        for arg in &self.parameters {
            write!(f, " {}", arg)?;
        }
        write!(f, " - {}", self.ty)
    }
}
