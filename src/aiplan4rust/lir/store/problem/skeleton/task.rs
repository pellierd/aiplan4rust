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

use crate::aiplan4rust::lang::{TaskSymbolId, TypeId, TypedList, VariableId};
use crate::aiplan4rust::lir::store::problem::skeleton::NamedTypedList;
use crate::aiplan4rust::lir::store::problem::SymbolRegistry;
use crate::aiplan4rust::lir::store::renderers;
use crate::aiplan4rust::lir::store::renderers::{
    LiftedDebugDisplay, LiftedSyntaxDisplay, RenderContext,
};
use core::fmt::{Debug, Formatter};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// Underlying signature containing the name and parameters.
    header: NamedTypedList<TaskSymbolId>,
    variable_symbols: SymbolRegistry<VariableId>,
}

impl Task {
    /// Creates a new `Task` with the specified name and parameters.
    ///
    /// # Parameters
    ///
    /// - `name`: The identifier for this task.
    /// - `parameters`: A typed list describing the task's parameters.
    pub fn new(task_symbol: TaskSymbolId, parameters: TypedList<VariableId, TypeId>) -> Self {
        let header = NamedTypedList::new(task_symbol, parameters);
        Self {
            header,
            variable_symbols: SymbolRegistry::new(),
        }
    }

    /// Permet d'ajouter les symboles après la création de manière élégante.
    /// Usage : Action::new_simple(...).with_symbols(ma_table)
    pub fn with_variable_symbols(mut self, symbols: SymbolRegistry<VariableId>) -> Self {
        self.variable_symbols = symbols;
        self
    }

    pub fn task_symbol(&self) -> TaskSymbolId {
        self.header.symbol()
    }

    /// Accès en lecture seule à la table des noms (symboles) des variables.
    /// À utiliser pour le rendu ou les messages d'erreur.
    pub fn variable_symbols(&self) -> &SymbolRegistry<VariableId> {
        &self.variable_symbols
    }

    /// Accès mutable à la table des noms des variables.
    pub fn variable_symbols_mut(&mut self) -> &mut SymbolRegistry<VariableId> {
        &mut self.variable_symbols
    }
}

impl Deref for Task {
    type Target = NamedTypedList<TaskSymbolId>;

    fn deref(&self) -> &Self::Target {
        &self.header
    }
}

impl DerefMut for Task {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.header
    }
}

impl LiftedSyntaxDisplay for Task {
    /// Rendu syntaxique HDDL : (:task move :parameters (?obj - object ?to - location))
    fn fmt_syntax(&self, f: &mut fmt::Formatter<'_>, ctx: &RenderContext) -> fmt::Result {
        let local_ctx = ctx.with_variables(&self.variable_symbols);
        renderers::syntax::task::render(f, self, &local_ctx)
    }
}

impl LiftedDebugDisplay for Task {
    /// Rendu technique : TaskSkeleton{ (move [t#4] ?obj [v#0] - object [t#1]) }
    fn fmt_debug(&self, f: &mut Formatter<'_>, ctx: &RenderContext) -> fmt::Result {
        let local_ctx = ctx.with_variables(&self.variable_symbols);
        renderers::debug::task::render(f, self, &local_ctx)
    }
}
