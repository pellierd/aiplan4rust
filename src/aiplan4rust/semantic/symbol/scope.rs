//! Module `scope`.
//!
//! This module defines the [`Scope`] struct, representing a lexical or semantic scope within
//! an Abstract Syntax Tree (AST). The scope is modeled as a stack of [`NodeId`]s corresponding
//! to AST nodes that define nested scopes.
//!
//! Scopes can be nested by extending an existing scope's stack with additional nodes.
//! This allows tracking the hierarchical context in which symbols or expr occur.

use crate::aiplan4rust::tree::NodeId;
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::fmt;
use std::hash::Hash;

/// Represents a lexical or semantic scope within the AST.
///
/// A `Scope` tracks a stack of [`NodeId`]s corresponding to AST nodes that define
/// the current nested scope. Each `NodeId` identifies a syntax node that delimits
/// a scope (such as a function, block, or module).
///
/// # Fields
///
/// - `stack`: A vector of `NodeId`s ordered from outermost (first) to innermost (last).
///
/// # Example
///
/// ```rust
/// let root_scope = Scope::root();
/// let new_scope = Scope::new(NodeId::new(42), Some(&root_scope));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Scope {
    stack: Vec<NodeId>,
}

impl Scope {
    /// Creates a new `Scope` starting with the given AST syntax node ID.
    ///
    /// If a parent scope is provided, the new scope inherits all nodes from the
    /// parent's stack before adding the new syntax ID, effectively nesting scopes.
    ///
    /// # Arguments
    ///
    /// * `ast` - The AST syntax node ID to start this scope with.
    /// * `parent` - An optional reference to a parent `Scope` to inherit nodes from.
    ///
    /// # Returns
    ///
    /// A new `Scope` instance with the combined stack.
    ///
    /// # Example
    ///
    /// ```rust
    /// let root_scope = Scope::new(NodeId::new(1), None);
    /// let nested_scope = Scope::new(NodeId::new(2), Some(&root_scope));
    /// ```
    pub fn new(ast: NodeId, parent: Option<&Scope>) -> Self {
        if let Some(parent_scope) = parent {
            let mut stack = Vec::with_capacity(parent_scope.stack.len() + 1);
            stack.extend_from_slice(&parent_scope.stack);
            stack.push(ast);
            Scope { stack }
        } else {
            Scope { stack: vec![ast] }
        }
    }

    /// Checks whether this scope starts with the given `prefix` scope.
    ///
    /// This tests if the current scope's stack begins with all elements of
    /// the `prefix` scope's stack, in order.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The scope to test as a prefix.
    ///
    /// # Returns
    ///
    /// `true` if this scope starts with `prefix`, otherwise `false`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let a = Scope::new(NodeId::new(1), None);
    /// let b = Scope::new(NodeId::new(2), Some(&a));
    /// assert!(b.starts_with(&a));
    /// ```
    pub fn starts_with(&self, prefix: &Scope) -> bool {
        // 1. Comparaison de pointeurs (Fastest path)
        // Si c'est exactement le même objet en mémoire, c'est forcément vrai.
        if std::ptr::eq(self, prefix) {
            return true;
        }

        // 2. Comparaison des longueurs (O(1))
        // Un scope ne peut pas être le préfixe d'un autre s'il est plus long.
        let prefix_len = prefix.stack.len();
        let self_len = self.stack.len();

        if prefix_len > self_len {
            return false;
        }

        // 3. Cas particulier : Root
        // Si le préfixe est vide, il est techniquement le préfixe de tout.
        if prefix_len == 0 {
            return true;
        }

        // 4. Comparaison profonde (O(N))
        // On ne fait cette boucle que si les tests rapides ci-dessus ont échoué.
        self.stack.starts_with(&prefix.stack)
    }

    /// Returns an iter over the `NodeId`s contained in this scope.
    ///
    /// The iter yields references to the syntax node IDs from outermost to innermost.
    ///
    /// # Returns
    ///
    /// An iter over `&NodeId`.
    ///
    /// # Example
    ///
    /// ```rust
    /// for node_id in scope.iter() {
    ///     println!("{:?}", node_id);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &NodeId> {
        self.stack.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }
}

impl fmt::Display for Scope {
    /// Formats the `Scope` by displaying its stack of `NodeId`s.
    ///
    /// The output is a comma-separated list of node IDs enclosed in square brackets,
    /// ordered from outermost to innermost.
    ///
    /// # Example
    ///
    /// ```rust
    /// let scope = Scope { stack: vec![NodeId(1), NodeId(2), NodeId(3)] };
    /// println!("{}", scope); // prints: [1, 2, 3]
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;

        let mut first = true;
        for node_id in &self.stack {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", node_id.as_usize())?;
            first = false;
        }

        write!(f, "]")
    }
}
