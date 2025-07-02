use std::fmt;
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::{Ident, Type};
use crate::aiplan4rust::ir::atomic_skeleton::{Skeleton, Signature};
use crate::aiplan4rust::syntax::DisplaySyntax;

/// Represents a function declaration skeleton in PDDL.
///
/// Wraps an `AbstractSkeleton` which contains the name and signature (parameters + return type).
/// Provides convenient constructors and can be extended with function-specific methods.
///
/// # Examples
///
/// ```
/// let func = FunctionSkeleton::new_function(
///     Ident::new("my_function"),
///     vec![Type::Object, Type::Integer],
///     Type::Bool,
/// );
/// assert_eq!(func.name(), "my_function");
/// assert_eq!(func.arity(), 2);
/// assert_eq!(func.return_type(), Some(&Type::Bool));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionSkeleton {
    /// Inner abstract skeleton (name + signature)
    abstract_skeleton: Skeleton,
}

impl FunctionSkeleton {

    /// Creates a new `FunctionSkeleton` with the given name, parameter types and return type.
    pub fn new(name: Ident, parameters: Vec<Type>, return_type: Type) -> Self {
        let mut signature = Signature::new(parameters, Some(return_type));
        let abstract_skeleton = Skeleton::new(name, signature);
        Self { abstract_skeleton }
    }
}

// Deref to AbstractSkeleton to access name and signature methods transparently
impl Deref for FunctionSkeleton {
    type Target = Skeleton;

    fn deref(&self) -> &Self::Target {
        &self.abstract_skeleton
    }
}

impl DerefMut for FunctionSkeleton {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.abstract_skeleton
    }
}

/// Delegate Display to Skeleton
impl fmt::Display for FunctionSkeleton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.abstract_skeleton.fmt(f)
    }
}

/// Delegate DisplayWithInterner to Skeleton
impl DisplayWithInterner for FunctionSkeleton {
    fn fmt_with(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.abstract_skeleton.fmt_with(f, interner)
    }
}

/// Delegate DisplaySyntax to Skeleton
impl DisplaySyntax for FunctionSkeleton {
    fn fmt_syntax(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.abstract_skeleton.fmt_syntax(f, interner)
    }
}
