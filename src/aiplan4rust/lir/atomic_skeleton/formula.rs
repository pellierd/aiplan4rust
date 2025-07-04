use std::fmt;
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::{Ident, TypedList};
use crate::aiplan4rust::lir::NamedTypedList;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::tree::TreeArena;

/// Represents the signature of a PDDL predicate.
///
/// A `PredicateSkeleton` stores the predicate name and the types of its parameters.
/// Unlike functions, predicates always return a Boolean value, so their return type
/// is implicitly `None`.
///
/// This type internally uses [`NamedTypedList`] to factor out the shared representation
/// of the identifier and parameters.
///
/// # Example
///
/// ```
/// use aiplan4rust::lang::{Ident, Type, TypedList};
/// use aiplan4rust::ir::atomic_skeleton::predicate::PredicateSkeleton;
///
/// let pred = PredicateSkeleton::new(
///     Ident::new("at"),
///     TypedList::from(vec![Type::Object, Type::Location]),
/// );
/// assert_eq!(pred.signature().return_type(), None);
/// assert_eq!(pred.signature().arity(), 2);
/// ```
///
/// # Notes
///
/// - Implements [`Deref`] and [`DerefMut`] to access the underlying [`NamedTypedList`] transparently.
/// - Supports pretty-printing with or without an interner.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Formula {
    /// Underlying skeleton holding the identifier and parameters.
    header: NamedTypedList,
}

impl Formula {
    /// Creates a new predicate signature from a name and parameter types.
    ///
    /// # Parameters
    ///
    /// - `name`: The identifier of the predicate.
    /// - `parameters`: The list of typed parameters.
    ///
    /// The return type is always set to `None`.
    pub fn new(name: Ident, parameters: TypedList) -> Self {
        let signature = NamedTypedList::new(name, parameters);
        Self { header: signature }
    }
}

// Allow direct access to Skeleton methods
impl Deref for Formula {
    type Target = NamedTypedList;

    fn deref(&self) -> &Self::Target {
        &self.header
    }
}

impl DerefMut for Formula {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.header
    }
}

impl FromAst for Formula {
    /// Parses a `PredicateSkeleton` from the AST.
    ///
    /// Expects a node structure where:
    /// - Child 0 is the predicate identifier.
    /// - Child 1 is the typed parameter list.
    fn from_ast(node: &AstArenaNode, ast: &TreeArena<AstArenaNode>) -> Result<Self, ParserInternalError> {
        let header = NamedTypedList::from_ast(node, ast)?;
        Ok(Formula { header })
    }
}

/// Displays the predicate in a human-readable form.
///
/// Delegates formatting to the underlying `Skeleton`.
impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.header.fmt(f)
    }
}

/// Displays the predicate using the provided interner to resolve identifiers.
impl DisplayWithInterner for Formula {
    fn fmt_with(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.header.fmt_with(f, interner)
    }
}

/// Displays the predicate in a syntax-oriented format.
impl DisplaySyntax for Formula {
    fn fmt_syntax(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.header.fmt_syntax(f, interner)
    }
}
