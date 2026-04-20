//! Atom of the Expression Store representing a unique, interned logical expression.
//!
//! `ExprEntry` is the fundamental building block of the [`ExprStore`]. Unlike a traditional
//! AST node, an entry is **immutable** and **independent** of its parents. It follows
//! the Hash-Consing principle: if two expressions are structurally identical, they
//! share the same entry in the store.
//!
//! # Key Differences from AST Nodes
//! - **No Parent Pointers**: Since an entry can be shared by multiple parents (DAG),
//!   it cannot store a single parent ID.
//! - **Structural Identity**: Equality is based on content (`kind` and `children`).
//! - **Reference by ID**: Children are referenced via [`ExprId`], ensuring the store
//!   remains a flat, cache-efficient structure.

use crate::aiplan4rust::lir::store::id::ExprId;
use crate::aiplan4rust::lir::store::ExprEntryKind;
use serde::{Deserialize, Serialize};

/// An immutable entry within the [`ExprStore`].
///
/// This struct represents a canonical logical expression. It is designed to be
/// stored in a contiguous vector where its position determines its [`ExprId`].
///
/// # Design Notes
/// - **Immutability**: Once interned in the store, an `ExprEntry` should never be modified
///   to maintain the integrity of the Hash-Consing lookup table.
/// - **Flattened Structure**: By using [`ExprId`] for children, we transform a recursive
///   tree into a Directed Acyclic Graph (DAG) stored in a flat arena.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExprEntry {
    /// The specific type and semantic data of this expression (Predicate, And, Not, etc.).
    kind: ExprEntryKind,

    /// The list of children identifiers pointing back into the [`ExprStore`].
    children: Vec<ExprId>,
}

impl ExprEntry {
    /// Creates a new `ExprEntry`.
    ///
    /// Note: Usually, you should use `ExprStore::intern` rather than creating
    /// entries manually to ensure uniqueness.
    pub fn new(kind: ExprEntryKind, children: Vec<ExprId>) -> Self {
        Self { kind, children }
    }

    /// Returns a shared slice of the child IDs.
    pub fn children(&self) -> &[ExprId] {
        &self.children
    }

    /// Returns the number of immediate children (arity).
    pub fn arity(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if this expression has no children (e.g., a Constant or a Nullary Predicate).
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns the [`ExprId`] of the child at the given index, if it exists.
    pub fn get_child(&self, index: usize) -> Option<ExprId> {
        self.children.get(index).copied()
    }

    /// Provides a reference to the kind of the expression.
    pub fn kind(&self) -> &ExprEntryKind {
        &self.kind
    }
}
