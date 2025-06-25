use serde::{Deserialize, Serialize};
use crate::aiplan4rust::ir::r#type::Type;

/// Internal structure managing an ordered list of parameter types.
///
/// This struct is not exposed publicly to enforce encapsulation.
/// Both predicate and function signatures delegate parameter handling to this.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Signature {
    parameters: Vec<Type>,
}

impl Signature {
    /// Creates a new `Signature` from the given parameter types.
    ///
    /// # Arguments
    ///
    /// * `parameters` - A vector of types representing the ordered parameters.
    pub fn new(parameters: Vec<Type>) -> Self {
        Self { parameters }
    }

    /// Returns an immutable slice of the parameter types.
    pub fn parameters(&self) -> &[Type] {
        &self.parameters
    }

    /// Returns a mutable slice of the parameter types.
    pub fn parameters_mut(&mut self) -> &mut [Type] {
        &mut self.parameters
    }

    /// Returns an iterator over immutable references to the parameter types.
    pub fn iter(&self) -> impl Iterator<Item = &Type> {
        self.parameters.iter()
    }

    /// Returns an iterator over mutable references to the parameter types.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Type> {
        self.parameters.iter_mut()
    }

    /// Returns the number of parameters.
    pub fn arity(&self) -> usize {
        self.parameters.len()
    }

    /// Returns true if there are no parameters.
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
    }

    /// Returns the parameter type at the given index, if any.
    pub fn get(&self, index: usize) -> Option<&Type> {
        self.parameters.get(index)
    }

    /// Returns a mutable reference to the parameter type at the given index, if any.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Type> {
        self.parameters.get_mut(index)
    }
}



/
