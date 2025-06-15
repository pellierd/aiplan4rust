use crate::aiplan4rust::syntax::ast::AstNode;

/// An iterator for traversing an `AstNode` in pre-order (depth-first).
///
/// This iterator visits the root node first, then recursively its children
/// from left to right. It can be used to perform operations such as walking
/// the entire abstract syntax tree (AST) structure.
///
/// # Example
///
/// ```rust
/// let root = AstNode::new(...);
/// let iter = PreorderIterator::new(&root);
///
/// for node in iter {
///     println!("{:?}", node);
/// }
/// ```
pub struct PreorderIterator<'a> {
    stack: Vec<&'a AstNode>,
}

impl<'a> PreorderIterator<'a> {
    /// Creates a new `PreorderIterator` starting from the given root node.
    ///
    /// # Parameters
    ///
    /// - `root`: A reference to the root `AstNode` to begin traversal from.
    ///
    /// # Returns
    ///
    /// A `PreorderIterator` instance that will yield nodes in pre-order.
    pub fn new(root: &'a AstNode) -> Self {
        Self { stack: vec![root] }
    }
}

impl<'a> Iterator for PreorderIterator<'a> {
    type Item = &'a AstNode;

    /// Advances the iterator and returns the next node in pre-order.
    ///
    /// Returns `None` when all nodes have been visited.
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        for child in node.children().iter().rev() {
            self.stack.push(child);
        }
        Some(node)
    }
}
