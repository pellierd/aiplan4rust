use std::fmt;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::{Ident, TypedList};
use crate::aiplan4rust::syntax::DisplaySyntax;

/// Abstract skeleton common to both predicates and functions in PDDL.
///
/// Encapsulates the shared parts like the name and the signature.
/// This allows factorizing common behavior and fields.
///
/// # Examples
///
/// ```
/// let pred = Skeleton::new(Ident::new("at"), Signature::new(vec![Type::Object]));
/// println!("Name: {}", pred.name);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Skeleton {
    /// Name of the predicate or function.
    name: Ident,
    /// Signature describing parameter types and optional return type.
    parameters: TypedList,
}

impl Skeleton {
    /// Creates a new skeleton with a name and a list of parameters.
    pub fn new(name: Ident, parameters: TypedList) -> Self {
        Self { name, parameters }
    }

    /// Returns the name.
    pub fn name(&self) -> &Ident {
        &self.name
    }

    /// Returns the signature.
    pub fn parameters(&self) -> &TypedList {
        &self.parameters
    }

    /// Returns a mutable reference to the signature.
    pub fn parameters_mut(&mut self) -> &mut TypedList {
        &mut self.parameters
    }
}


impl Display for Skeleton {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "[name: ")?;
        self.name.fmt(f)?;
        write!(f, ", parameters: ")?;
        self.parameters.fmt(f)?;
        write!(f, "]")
    }
}

impl DisplayWithInterner for Skeleton {
    fn fmt_with(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        write!(f, "[name: ")?;
        self.name.fmt_with(f, interner)?;
        write!(f, ", parameters: ")?;
        self.parameters.fmt_with(f, interner)?;
        write!(f, "]")
    }
}

impl DisplaySyntax for Skeleton {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.fmt_with(f, interner)
    }
}
