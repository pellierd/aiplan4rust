use std::fmt;
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::{Ident, Type, TypedList};
use crate::aiplan4rust::ir::atomic_skeleton::Skeleton;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};

/// Represents the signature of a PDDL predicate.
///
/// A `PredicateSkeleton` stores the predicate name and the types of its parameters.
/// Unlike functions, predicates always return a Boolean value, so their return type
/// is implicitly `None`.
///
/// This type internally uses [`Skeleton`] to factor out the shared representation
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
/// - Implements [`Deref`] and [`DerefMut`] to access the underlying [`Skeleton`] transparently.
/// - Supports pretty-printing with or without an interner.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Predicate {
    /// Underlying skeleton holding the identifier and parameters.
    skeleton: Skeleton,
}

impl Predicate {
    /// Creates a new predicate signature from a name and parameter types.
    ///
    /// # Parameters
    ///
    /// - `name`: The identifier of the predicate.
    /// - `parameters`: The list of typed parameters.
    ///
    /// The return type is always set to `None`.
    pub fn new(name: Ident, parameters: TypedList) -> Self {
        let skeleton = Skeleton::new(name, parameters);
        Self { skeleton }
    }
}

// Allow direct access to Skeleton methods
impl Deref for Predicate {
    type Target = Skeleton;

    fn deref(&self) -> &Self::Target {
        &self.skeleton
    }
}

impl DerefMut for Predicate {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.skeleton
    }
}

impl FromAst for Predicate {
    /// Parses a `PredicateSkeleton` from the AST.
    ///
    /// Expects a node structure where:
    /// - Child 0 is the predicate identifier.
    /// - Child 1 is the typed parameter list.
    fn from_ast(node: &AstArenaNode, ast: &TreeArena<AstArenaNode>) -> Result<Self, ParserInternalError> {
        println!("@@@@@@{}", node);
        let predicate_id = node.try_child(0)?;
        let predicate_node = ast.try_node(predicate_id)?;
        let predicate = predicate_node.try_ident()?;

        let parameters_id = node.try_child(1)?;
        let parameters_node = ast.try_node(parameters_id)?;
        let parameters = TypedList::from_ast(parameters_node, ast)?;

        Ok(Predicate::new(predicate, parameters))
    }
}

/// Displays the predicate in a human-readable form.
///
/// Delegates formatting to the underlying `Skeleton`.
impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.skeleton.fmt(f)
    }
}

/// Displays the predicate using the provided interner to resolve identifiers.
impl DisplayWithInterner for Predicate {
    fn fmt_with(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.skeleton.fmt_with(f, interner)
    }
}

/// Displays the predicate in a syntax-oriented format.
impl DisplaySyntax for Predicate {
    fn fmt_syntax(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.skeleton.fmt_syntax(f, interner)
    }
}
