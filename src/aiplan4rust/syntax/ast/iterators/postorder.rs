use crate::aiplan4rust::syntax::ast::AstNode;

/// An iterator for traversing an `AstNode` in post-order (depth-first).
///
/// This iterator visits all the children of a node before visiting the node itself.
/// It is commonly used when operations need to be performed bottom-up, such as
/// evaluating expressions or freeing resources.
///
/// # Example
///
/// ```rust
/// let root = AstNode::new(...);
/// let iter = PostorderIterator::new(&root);
///
/// for node in iter {
///     println!("{:?}", node);
/// }
/// ```
pub struct PostorderIterator<'a> {
    stack: Vec<(&'a AstNode, bool)>,
}

impl<'a> PostorderIterator<'a> {
    /// Creates a new `PostorderIterator` starting from the given root node.
    pub fn new(root: &'a AstNode) -> Self {
        Self {
            stack: vec![(root, false)],
        }
    }
}

impl<'a> Iterator for PostorderIterator<'a> {
    type Item = &'a AstNode;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node, visited)) = self.stack.pop() {
            if visited {
                return Some(node);
            } else {
                // Push the node back with visited = true,
                // then push children (to be visited first)
                self.stack.push((node, true));
                for child in node.children().iter().rev() {
                    self.stack.push((child, false));
                }
            }
        }
        None
    }
}
