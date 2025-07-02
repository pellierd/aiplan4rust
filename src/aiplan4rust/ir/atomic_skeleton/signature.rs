use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::slice::{Iter, IterMut};
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::syntax::DisplaySyntax;

/// Internal structure managing an ordered list of parameter types.
///
/// This struct encapsulates the list of parameter types used in predicate
/// and function signatures. It is not exposed publicly to enforce
/// encapsulation and allow internal evolution without breaking API.
///
/// All typical read-only slice operations (indexing, iteration) are
/// available via dereferencing.
///
/// # Example
///
/// ```
/// let sig = Signature::new(vec![Type::Integer, Type::Bool]);
/// assert_eq!(sig.arity(), 2);
/// assert_eq!(sig[0], Type::Integer);
/// for param in &sig {
///     println!("{:?}", param);
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Signature {
    /// The list of parameter types in order.
    parameters: Vec<Type>,
    /// Optional return type. `None` for predicates, `Some(type)` for functions.
    return_type: Option<Type>,
}

impl Signature {

    /// Creates a new `Signature` with given parameters and a return type.
    ///
    /// # Arguments
    ///
    /// * `parameters` - A vector containing the types of the parameters, in order.
    /// * `return_type` - The return type of the signature.
    ///
    /// # Returns
    ///
    /// A new `Signature` instance with the given return type.
    pub fn new(parameters: Vec<Type>, return_type: Option<Type>) -> Self {
        Self {
            parameters,
            return_type,
        }
    }

    /// Returns the number of parameters.
    pub fn arity(&self) -> usize {
        self.parameters.len()
    }

    /// Returns an immutable slice of the parameter types.
    pub fn parameters(&self) -> &[Type] {
        &self.parameters
    }

    /// Returns a mutable slice of the parameter types.
    ///
    /// Mutation through this slice should be done carefully to preserve invariants.
    pub fn parameters_mut(&mut self) -> &mut [Type] {
        &mut self.parameters
    }

    /// Checks if there are no parameters.
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

    /// Returns a reference to the return type, if any.
    pub fn return_type(&self) -> Option<&Type> {
        self.return_type.as_ref()
    }

    /// Returns a mutable reference to the return type, if any.
    pub fn return_type_mut(&mut self) -> Option<&mut Type> {
        self.return_type.as_mut()
    }

    /// Sets the return type.
    pub fn set_return_type(&mut self, return_type: Type) {
        self.return_type = Some(return_type);
    }
}

impl Deref for Signature {
    type Target = [Type];

    /// Dereferences to a slice of parameter types for read-only access.
    fn deref(&self) -> &Self::Target {
        &self.parameters
    }
}

impl IntoIterator for Signature {
    type Item = Type;
    type IntoIter = std::vec::IntoIter<Type>;

    /// Consumes the signature and returns an iterator over owned parameter types.
    fn into_iter(self) -> Self::IntoIter {
        self.parameters.into_iter()
    }
}

impl<'a> IntoIterator for &'a Signature {
    type Item = &'a Type;
    type IntoIter = Iter<'a, Type>;

    /// Returns an iterator over immutable references to parameter types.
    fn into_iter(self) -> Self::IntoIter {
        self.parameters.iter()
    }
}

impl<'a> IntoIterator for &'a mut Signature {
    type Item = &'a mut Type;
    type IntoIter = IterMut<'a, Type>;

    /// Returns an iterator over mutable references to parameter types.
    fn into_iter(self) -> Self::IntoIter {
        self.parameters.iter_mut()
    }
}

impl Display for Signature {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Signature {{ parameters: [")?;
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "?X{} - {}", i, param)?;
        }
        write!(f, "]")?;
        if let Some(ref return_type) = self.return_type {
            write!(f, ", return: {} }}", return_type)
        } else {
            write!(f, ", return: None }}")
        }
    }
}

impl DisplayWithInterner for Signature {
    fn fmt_with(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> std::fmt::Result {
        write!(f, "Signature {{ parameters: [")?;
        for (i, param) in self.parameters.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "?X{} - {}", i, param.to_string_with_interner(interner))?;
        }
        write!(f, "]")?;
        if let Some(ref return_type) = self.return_type {
            write!(f, ", return: {} }}", return_type.to_string_with_interner(interner))
        } else {
            write!(f, ", return: None }}")
        }
    }
}

impl DisplaySyntax for Signature {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> std::fmt::Result {
        self.fmt_with(f, interner)
    }
}
