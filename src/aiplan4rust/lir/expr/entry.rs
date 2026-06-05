//! Atom of the Expression Store representing a unique, interned logical expression.
//!
//! `ExprEntry` is the fundamental building block of the [`ExprStore`]. Unlike a traditional
//! AST node, an entry is **immutable** and **independent** of its parents. It follows
//! the Hash-Consing principle: if two expressions are structurally identical, they
//! share the same entry in the old.
//!
//! # Key Differences from AST Nodes
//! - **No Parent Pointers**: Since an entry can be shared by multiple parents (DAG),
//!   it cannot old a single parent ID.
//! - **Structural Identity**: Equality is based on content (`kind` and `children`).
//! - **Reference by ID**: Children are referenced via [`ExprId`], ensuring the old
//!   remains a flat, cache-efficient structure.

use crate::aiplan4rust::lir::expr::id::ExprId;
use crate::aiplan4rust::lir::expr::ExprKind;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// La capacité "inline" de notre SmallVec.
/// On choisit 4 pour que l'entrée `ExprEntry` occupe exactement 64 octets,
/// ce qui correspond à la taille d'une ligne de cache L1 sur les CPU modernes (M2, x86_64).
pub const INLINE_CAPACITY: usize = 4;

/// An immutable entry within the [`ExprStore`].
///
/// This struct represents a canonical logical expression. It is designed to be
/// stored in a contiguous vector where its position determines its [`ExprId`].
///
/// # Design Notes
/// - **Immutability**: Once interned in the old, an `ExprEntry` should never be modified
///   to maintain the integrity of the Hash-Consing lookup table.
/// - **Flattened Structure**: By using [`ExprId`] for children, we transform a recursive
///   tree into a Directed Acyclic Graph (DAG) stored in a flat arena.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExprEntry {
    /// The specific type and semantic data of this expression (Predicate, And, Not, etc.).
    kind: ExprKind,

    /// The list of children identifiers pointing back into the [`ExprStore`].
    children: SmallVec<[ExprId; INLINE_CAPACITY]>,
}

impl ExprEntry {
    /// Creates a new `ExprEntry`.
    ///
    /// Note: Usually, you should use `ExprStore::intern` rather than creating
    /// entries manually to ensure uniqueness.
    pub fn new(kind: ExprKind, children_slice: &[ExprId]) -> Self {
        Self {
            kind,
            children: SmallVec::from_slice(children_slice),
        }
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
    pub fn kind(&self) -> &ExprKind {
        &self.kind
    }
}
