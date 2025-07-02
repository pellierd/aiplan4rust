use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::ir::atomic_skeleton::Signature;

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
    pub name: Ident,
    /// Signature describing parameter types and optional return type.
    pub signature: Signature,
}

impl Skeleton {
    /// Creates a new skeleton with a name and a signature.
    pub fn new(name: Ident, signature: Signature) -> Self {
        Self { name, signature }
    }

    /// Returns the name.
    pub fn name(&self) -> &Ident {
        &self.name
    }

    /// Returns the signature.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Returns a mutable reference to the signature.
    pub fn signature_mut(&mut self) -> &mut Signature {
        &mut self.signature
    }
}

use std::fmt::{Display, Formatter, Result as FmtResult};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::ir::atomic_skeleton::Signature;
use crate::aiplan4rust::interner::{StringInterner, DisplayWithInterner, DisplaySyntax};
use crate::aiplan4rust::syntax::DisplaySyntax;

impl Display for Skeleton {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Skeleton {{ name: {}, ", self.name)?;
        write!(f, "signature: {} }}", self.signature)
    }
}

impl DisplayWithInterner for Skeleton {
    fn fmt_with(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> FmtResult {
        write!(
            f,
            "Skeleton {{ name: {}, signature: ",
            self.name.to_string_with_interner(interner)
        )?;
        self.signature.fmt_with(f, interner)?;
        write!(f, " }}")
    }
}

impl DisplaySyntax for Skeleton {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> FmtResult {
        self.fmt_with(f, interner)
    }
}
