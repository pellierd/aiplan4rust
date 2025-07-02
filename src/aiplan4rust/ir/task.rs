use std::fmt;
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::{Ident, Type, TypedList};
use crate::aiplan4rust::ir::{Function, Signature};
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};

/// Represents a planning task in HDDL.
///
/// A task is defined by a unique name and a list of typed parameters.
/// It typically corresponds to a high-level goal to be decomposed into subtasks or actions.
///
/// # Examples
///
/// ```rust
/// let name = Ident::new("move".to_string());
/// let parameters = TypedList::new(); // or fill with parameters
/// let task = Task::new(name, parameters);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Task {
    signature: Signature,
}

impl Task {
    /// Creates a new `Task` with the given name and parameters.
    ///
    /// # Arguments
    ///
    /// * `name` - The identifier representing the name of the task.
    /// * `parameters` - The typed parameter list associated with this task.
    ///
    /// # Returns
    ///
    /// A new `Task` instance.
    pub fn new(name: Ident, parameters: TypedList) -> Self {
        let signature = Signature::new(name, parameters);
        Self { signature }
    }
}

// Allow direct access to the methods of `Signature`.
impl Deref for Task {
    type Target = Signature;

    fn deref(&self) -> &Self::Target {
        &self.signature
    }
}

impl DerefMut for Task {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.signature
    }
}

impl FromAst for Task {
    /// Builds a `Task` instance from the abstract syntax tree (AST).
    ///
    /// Expects a node structure where:
    /// - Child 0 is the task name identifier.
    /// - Child 1 is the typed parameter list.
    ///
    /// # Errors
    ///
    /// Returns `ParserInternalError` if the node is malformed or identifiers are missing.
    fn from_ast(
        node: &AstArenaNode,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<Self, ParserInternalError> {
        let signature = Signature::from_ast(node, ast)?;
        Ok(Task { signature })
    }
}

/// Displays the task in a human-readable form.
///
/// This delegates formatting to the underlying `Signature`.
impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.signature.fmt(f)
    }
}

/// Displays the task using the provided interner to resolve identifiers.
///
/// This allows identifiers to be shown as their original strings.
impl DisplayWithInterner for Task {
    fn fmt_with(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.signature.fmt_with(f, interner)
    }
}

/// Displays the task in a syntax-oriented format.
///
/// This is useful for reconstructing the source representation.
impl DisplaySyntax for Task {
    fn fmt_syntax(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.signature.fmt_syntax(f, interner)
    }
}
