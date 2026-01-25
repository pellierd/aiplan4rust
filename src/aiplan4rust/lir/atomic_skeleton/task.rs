//! Task Signature Representation (`AtomicTaskSkeleton`)
//!
//! This module defines the [`Task`] struct, which represents the signature of a high-level
//! syntax task in HDDL. Tasks have a name and typed parameters, but no return type_checker.
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

use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{Ident, TypedList};
use crate::aiplan4rust::lir::atomic_skeleton::NamedTypedList;
use crate::aiplan4rust::syntax::SyntaxInternerDisplay;

/// Represents a syntax task declaration in HDDL.
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
/// - Can be created from an AST syntax via [`FromAst`].
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

    /// Creates a new `Task` from an already constructed task header.
    ///
    /// This constructor is intended for **internal use only** within the crate.
    /// It allows creating a `Task` by taking ownership of an existing
    /// [`NamedTypedList`], avoiding the need to deconstruct and rebuild the
    /// signature during the encoding process.
    ///
    /// # Arguments
    ///
    /// * `header` - A fully constructed task header (name and parameters).
    ///
    /// # Returns
    ///
    /// A new `Task` instance.
    ///
    /// # Notes
    ///
    /// This function is marked `pub(crate)` as it is a specialized constructor
    /// for the LIR translation layer and should not be used by external consumers.
    pub(crate) fn from_header(header: NamedTypedList) -> Self {
        Self { header }
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

impl fmt::Display for Task {
    /// Formats the task into a human-readable string.
    ///
    /// This delegates to the `Display` implementation of the underlying `NamedTypedList`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.header.fmt(f)
    }
}

impl InternerDisplay for Task {
    /// Formats the task using the provided interner to resolve identifier names.
    ///
    /// This allows rendering identifiers as their original strings instead of numeric IDs.
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
    ) -> fmt::Result {
        self.header.fmt_with_interner(f, interner)
    }
}

impl SyntaxInternerDisplay for Task {
    /// Formats the task in a syntax-oriented representation.
    ///
    /// This can be used to reconstruct or pretty-print the original declaration.
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        self.header.fmt_syntax_with_interner_and_indent(f, interner, indent)
    }
}
