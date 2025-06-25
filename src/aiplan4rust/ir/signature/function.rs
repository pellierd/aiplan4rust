// Represents the signature of a function: an ordered list of parameter types
/// plus a return type.
///
/// This struct provides controlled access to both parameters and return type,
/// and can be extended with function-specific methods if needed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionSignature {
    signature: Signature,
    return_type: Type,
}

impl FunctionSignature {
    /// Creates a new function signature with the given parameter types and return type.
    ///
    /// # Arguments
    ///
    /// * `parameters` - A vector of types for the function parameters.
    /// * `return_type` - The return type of the function.
    pub fn new(parameters: Vec<Type>, return_type: Type) -> Self {
        Self { signature: Signature::new(parameters), return_type }
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

    /// Returns a reference to the return type.
    pub fn return_type(&self) -> &Type {
        &self.return_type
    }

    /// Returns a mutable reference to the return type.
    pub fn return_type_mut(&mut self) -> &mut Type {
        &mut self.return_type
    }
}
