use crate::aiplan4rust::semantic::arena::{ArenaAst, NodeId};
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::frontend::ParserInternalError;

use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::fmt;
use std::hash::Hash;
use once_cell::sync::Lazy;

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
    /// Creates a new `Scope` starting with the given AST node ID.
    ///
    /// If a parent scope is provided, the new scope inherits all node IDs from
    /// the parent's stack before adding the new node ID.
    ///
    /// # Parameters
    /// - `ast`: The AST node ID to start the scope with.
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

    /// Returns a reference to the root `Scope`.
    ///
    /// The root scope contains only the root node ID (`NodeId::ROOT_NODE_ID`).
    ///
    /// # Returns
    /// A static reference to the root `Scope`.
    ///
    /// # Example
    /// ```rust
    /// let root = Scope::root();
    /// ```
    pub fn root() -> &'static Scope {
        static ROOT: Lazy<Scope> = Lazy::new(|| {
            Scope {
                stack: vec![NodeId::ROOT_NODE_ID], // définition directe ici
            }
        });
        &ROOT
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

    /// Returns an iterator over the node IDs contained in the scope.
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

    /// Checks if the scope contains at least one AST node of the specified kind.
    ///
    /// This function iterates over the node IDs stored in the scope's `stack` and
    /// attempts to retrieve each corresponding AST node using `try_node`. If any
    /// node is not found, it returns a `ParserInternalError`, indicating an internal
    /// inconsistency.
    ///
    /// If at least one node matches the specified kind, the function returns `Ok(true)`.
    /// If no nodes match, it returns `Ok(false)`.
    ///
    /// # Parameters
    /// - `kind`: The kind of AST node to search for.
    /// - `ast`: Reference to the AST arena containing all nodes.
    ///
    /// # Returns
    /// - `Ok(true)` if any node in the scope's stack has the specified kind.
    /// - `Ok(false)` if no node matches the specified kind.
    /// - `Err(ParserInternalError)` if an expected node ID is not found.
    ///
    /// # Example
    /// ```rust
    /// let result = scope.contains_node_of_kind(AstKind::Function, &ast);
    /// match result {
    ///     Ok(true) => println!("Scope contains a function node."),
    ///     Ok(false) => println!("Scope does not contain a function node."),
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    pub fn contains_node_of_kind(
        &self,
        kind: AstKind,
        ast: &ArenaAst,
    ) -> Result<bool, ParserInternalError> {
        for &id in self.iter() {
            let node = ast.try_node(id)?;
            if *node.kind() == kind {
                return Ok(true);
            }
        }
        Ok(false)
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
            write!(f, "{}", node_id)?;
            first = false;
        }

        // Close the list with a closing bracket
        write!(f, "]")
    }
}
