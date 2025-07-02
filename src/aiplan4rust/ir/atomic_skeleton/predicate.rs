use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::{Ident, Type};
use crate::aiplan4rust::ir::atomic_skeleton::{Skeleton, Signature};
use crate::aiplan4rust::ir::atomic_skeleton::function::FunctionSkeleton;
use crate::aiplan4rust::syntax::DisplaySyntax;

/// Skeleton représentant un prédicat PDDL.
///
/// Utilise `AbstractSkeleton` pour factoriser le nom et la signature.
///
/// Le `return_type` dans la signature est toujours `None` pour un prédicat.
///
/// # Exemple
///
/// ```
/// let pred = PredicateSkeleton::new(Ident::new("at"), vec![Type::Object, Type::Location]);
/// assert_eq!(pred.signature().return_type(), None);
/// assert_eq!(pred.signature().arity(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PredicateSkeleton {
    pub abstract_skeleton: Skeleton,
}

impl PredicateSkeleton {
    /// Crée un nouveau prédicat avec un nom et une liste de types de paramètres.
    ///
    /// Le `return_type` est forcé à `None` pour les prédicats.
    pub fn new(name: Ident, parameters: Vec<Type>) -> Self {
        let signature = Signature::new(parameters, None);
        let abstract_skeleton = Skeleton::new(name, signature);
        Self { abstract_skeleton }
    }

    /// Retourne une référence au nom du prédicat.
    pub fn name(&self) -> &Ident {
        &self.abstract_skeleton.name
    }

    /// Retourne une référence à la signature du prédicat.
    pub fn signature(&self) -> &Signature {
        &self.abstract_skeleton.signature
    }

    /// Retourne une référence mutable à la signature.
    pub fn signature_mut(&mut self) -> &mut Signature {
        &mut self.abstract_skeleton.signature
    }
}

/// Delegate Display to Skeleton
impl fmt::Display for PredicateSkeleton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.abstract_skeleton.fmt(f)
    }
}

/// Delegate DisplayWithInterner to Skeleton
impl DisplayWithInterner for PredicateSkeleton {
    fn fmt_with(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.abstract_skeleton.fmt_with(f, interner)
    }
}

/// Delegate DisplaySyntax to Skeleton
impl DisplaySyntax for PredicateSkeleton {
    fn fmt_syntax(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.abstract_skeleton.fmt_syntax(f, interner)
    }
}
