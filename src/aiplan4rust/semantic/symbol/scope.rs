use crate::aiplan4rust::syntax::tree::NodeId;

use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::fmt;
use std::hash::Hash;

/// Represents a lexical or semantic scope within the AST.
///
/// The `Scope` struct tracks a stack of `NodeId`s corresponding to AST nodes that
/// define the current scope. Scopes can be nested by extending the stack with nodes
/// from parent scopes.
///
/// # Fields
///
/// * `stack` - A vector of `NodeId`s representing the nodes included in this scope,
///   ordered from outermost to innermost.
///
/// # Example
///
/// ```rust
/// let root_scope = Scope::root();
/// let new_scope = Scope::new(NodeId::new(42), Some(root_scope));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Scope {
    stack: Vec<NodeId>,
}

impl Scope {
    /// Creates a new `Scope` starting with the given AST syntax ID.
    ///
    /// If a parent scope is provided, the new scope inherits all syntax IDs from
    /// the parent's stack before adding the new syntax ID.
    ///
    /// # Parameters
    /// - `ast`: The AST syntax ID to start the scope with.
    /// - `parent`: An optional reference to a parent scope to inherit from.
    ///
    /// # Returns
    /// A new `Scope` instance.
    ///
    /// # Example
    /// ```rust
    /// let root_scope = Scope::root();
    /// let new_scope = Scope::new(NodeId::new(42), Some(root_scope));
    /// ```
    pub fn new(ast: NodeId, parent: Option<&Scope>) -> Self {
        let mut scope = Scope { stack: Vec::new() };

        if let Some(parent_scope) = parent {
            scope.stack.extend(parent_scope.stack.iter().cloned());
        }
        scope.stack.push(ast);
        scope
    }

    /// Checks if `self` scope starts with the given `prefix` scope.
    ///
    /// This compares the internal `stack` vectors to see if `self` begins with
    /// all elements of `prefix` in order.
    ///
    /// # Parameters
    /// - `prefix`: The scope to check as a prefix.
    ///
    /// # Returns
    /// `true` if `self` starts with `prefix`, otherwise `false`.
    ///
    /// # Example
    /// ```rust
    /// let a = Scope::new(NodeId::new(1), None);
    /// let b = Scope::new(NodeId::new(2), Some(&a));
    /// assert!(b.starts_with(&a));
    /// ```
    pub fn starts_with(&self, prefix: &Scope) -> bool {
        self.stack.starts_with(&prefix.stack)
    }

    /// Returns an iterator over the syntax IDs contained in the scope.
    ///
    /// # Returns
    /// An iterator yielding references to `NodeId`s in the scope.
    ///
    /// # Example
    /// ```rust
    /// for node_id in scope.iter() {
    ///     println!("{:?}", node_id);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &NodeId> {
        self.stack.iter()
    }

}

/// Implements the iterator trait for the `Scope` struct.
///
/// This implementation allows iterating over the `NodeId`s stored in the `stack` vector,
/// by popping and returning the last element on each call to `next()`.
/// Thus, the iteration proceeds from the most recently added element to the oldest.
///
/// # Note
///
/// This iteration modifies the internal `stack` by removing elements as they are iterated over.
/// After a complete iteration, the `stack` will be empty.
///
/// # Example
///
/// ```rust
/// let mut scope = Scope::new(NodeId::new(1), None);
/// scope.stack.push(NodeId::new(2));
/// while let Some(node_id) = scope.next() {
///     println!("NodeId: {:?}", node_id);
/// }
/// // After the loop, scope.stack is empty.
/// ```
impl Iterator for Scope {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop()
    }
}

impl fmt::Display for Scope {
    /// Formats the `Scope` by displaying its stack of `NodeId`s.
    ///
    /// Each `NodeId` is displayed using its own `Display` implementation.
    /// The `NodeId`s are printed in order, separated by commas and enclosed in square brackets.
    ///
    /// # Example
    /// ```rust
    /// let scope = Scope { stack: vec![NodeId(1), NodeId(2), NodeId(3)] };
    /// println!("{}", scope); // prints: [1, 2, 3]
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Start the list with an opening bracket
        write!(f, "[")?;

        let mut first = true;

        // Iterate over all NodeIds in the stack
        for node_id in &self.stack {
            // Add a comma before each item except the first
            if !first {
                write!(f, ", ")?;
            }
            // Write the NodeId using its Display implementation
            write!(f, "{}", node_id.as_usize())?;
            first = false;
        }

        // Close the list with a closing bracket
        write!(f, "]")
    }
}
