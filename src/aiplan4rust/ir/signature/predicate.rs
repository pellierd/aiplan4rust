/// Represents the signature of a predicate: an ordered list of parameter types.
///
/// This struct provides controlled access to the parameter types and
/// can be extended with predicate-specific methods if needed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PredicateSignature {
    signature: Signature,
}

impl PredicateSignature {
    /// Creates a new predicate signature with the given parameter types.
    ///
    /// # Arguments
    ///
    /// * `parameters` - A vector of types for the predicate parameters.
    pub fn new(parameters: Vec<Type>) -> Self {
        Self { signature: Signature::new(parameters) }
    }

    /// Returns an immutable slice of the parameter types.
    pub fn parameters(&self) -> &[Type] {
        self.signature.parameters()
    }

    /// Returns a mutable slice of the parameter types.
    pub fn parameters_mut(&mut self) -> &mut [Type] {
        self.signature.parameters_mut()
    }

    /// Returns an iterator over immutable references to the parameter types.
    pub fn iter(&self) -> impl Iterator<Item = &Type> {
        self.signature.iter()
    }

    /// Returns an iterator over mutable references to the parameter types.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Type> {
        self.signature.iter_mut()
    }

    /// Returns the number of parameters.
    pub fn arity(&self) -> usize {
        self.signature.arity()
    }

    /// Returns true if there are no parameters.
    pub fn is_empty(&self) -> bool {
        self.signature.is_empty()
    }

    /// Returns the parameter type at the given index, if any.
    pub fn get(&self, index: usize) -> Option<&Type> {
        self.signature.get(index)
    }

    /// Returns a mutable reference to the parameter type at the given index, if any.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Type> {
        self.signature.get_mut(index)
    }
}
