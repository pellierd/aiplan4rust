//! Task Signature Representation (`AtomicTaskSkeleton`)
//!
//! This module defines the [`Task`] struct, which represents the signature of a high-level
//! planning task in HDDL. Tasks have a name and typed parameters, but no return type.
//!
//! This structure is re-exported as [`AtomicTaskSkeleton`] from the parent module.
//!
//! # Example Use
//!
//! ```rust
//! use aiplan4rust::lir::atomic_skeleton::AtomicTaskSkeleton;
//! use aiplan4rust::lang::{Ident, TypedList};
//!
//! let task = AtomicTaskSkeleton::new(Ident::new("move"), TypedList::empty());
//! ```

use std::fmt;
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lang::{Ident, TypedList};
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::syntax::PlanningSyntaxDisplay;
use crate::aiplan4rust::tree::TreeArena;

/// Represents a planning task declaration in HDDL.
///
/// A `Task` has:
/// - A name (identifier)
/// - A typed parameter list (its arguments)
///
/// This is the high-level structure describing the signature of a compound or primitive task.
///
/// # Example
///
/// ```rust
/// use aiplan4rust::lang::{Ident, TypedList};
/// use aiplan4rust::lir::atomic_skeleton::task::Task;
///
/// let task = Task::new(
///     Ident::new("move"),
///     TypedList::from(vec![])
/// );
/// assert_eq!(task.name().as_str(), "move");
/// ```
///
/// # Notes
///
/// - Implements [`Deref`] and [`DerefMut`] to expose the underlying [`NamedTypedList`] transparently.
/// - Can be created from an AST node via [`FromAst`].
/// - Supports pretty-printing and interner-based rendering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Task {
    /// Underlying signature containing the name and parameters.
    header: NamedTypedList,
}

impl Task {
    /// Creates a new `Task` with the specified name and parameters.
    ///
    /// # Parameters
    ///
    /// - `name`: The identifier for this task.
    /// - `parameters`: A typed list describing the task's parameters.
    pub fn new(name: Ident, parameters: TypedList) -> Self {
        let signature = NamedTypedList::new(name, parameters);
        Self { header: signature }
    }
}

impl Deref for Task {
    type Target = NamedTypedList;

    fn deref(&self) -> &Self::Target {
        &self.header
    }
}

impl DerefMut for Task {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.header
    }
}

impl FromAst for Task {
    /// Builds a `Task` from an abstract syntax tree node.
    ///
    /// The node is expected to have:
    /// - Child 0: The identifier (name).
    /// - Child 1: The typed parameter list.
    ///
    /// # Errors
    ///
    /// Returns [`ParserInternalError`] if the node is malformed or required children are missing.
    fn from_ast(
        node: &AstArenaNode,
        ast: &TreeArena<AstArenaNode>,
    ) -> Result<Self, ParserInternalError> {
        let signature = NamedTypedList::from_ast(node, ast)?;
        Ok(Task { header: signature })
    }
}

impl fmt::Display for Task {
    /// Formats the task into a human-readable string.
    ///
    /// This delegates to the `Display` implementation of the underlying `NamedTypedList`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.header.fmt(f)
    }
}

impl DisplayWithInterner for Task {
    /// Formats the task using the provided interner to resolve identifier names.
    ///
    /// This allows rendering identifiers as their original strings instead of numeric IDs.
    fn fmt_with(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.header.fmt_with(f, interner)
    }
}

impl PlanningSyntaxDisplay for Task {
    /// Formats the task in a syntax-oriented representation.
    ///
    /// This can be used to reconstruct or pretty-print the original declaration.
    fn fmt_planning_syntax_with_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        self.header.fmt_planning_syntax_with_indent(f, interner, indent)
    }
}
