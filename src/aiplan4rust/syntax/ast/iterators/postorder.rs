use crate::aiplan4rust::syntax::ast::AstNode;

/// An iterator that traverses an `IntAstNode` tree in post-order (depth-first) and yields each node along with its depth.
///
/// In a post-order traversal, all children of a node are visited before the node itself.
/// This is particularly useful in scenarios such as evaluating expressions or performing
/// cleanup tasks where dependencies must be processed before their parent node.
///
/// # Fields
/// - `stack`: A manual stack used to simulate recursion. Each element contains a tuple:
///   (`&IntAstNode`, depth: `usize`, visited: `bool`). The `visited` flag indicates
///   whether the node has already been processed after its children.
///
/// # Example
/// ```rust
/// use aiplan4rust::syntax::int_ast::{IntAstNode, PostorderIter};
///
/// let root: &IntAstNode = ...; // assume you have a root node
/// let iter = PostorderIter::new(root);
///
/// for (node, depth) in iter {
///     println!("{}- {:?}", "  ".repeat(depth), node.kind());
/// }
/// ```
pub struct PostorderIter<'a> {
    /// The traversal stack, containing tuples of the node, its depth, and whether it has been visited.
    stack: Vec<(&'a AstNode, usize, bool)>,
}

impl<'a> PostorderIter<'a> {
    /// Constructs a new `PostorderIter` starting from the given root node.
    ///
    /// # Arguments
    /// * `root` - A reference to the root of the AST subtree to traverse.
    ///
    /// # Returns
    /// A `PostorderIter` ready to iterate over the subtree in post-order.
    pub fn new(root: &'a AstNode) -> Self {
        Self {
            stack: vec![(root, 0, false)],
        }
    }
}

impl<'a> Iterator for PostorderIter<'a> {
    type Item = (&'a AstNode, usize);

    /// Returns the next node and its depth in post-order traversal.
    ///
    /// If the traversal is complete, `None` is returned.
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node, depth, visited)) = self.stack.pop() {
            if visited {
                // The node has already had its children visited, yield it now.
                return Some((node, depth));
            } else {
                // Mark the node to be revisited after its children.
                self.stack.push((node, depth, true));

                // Push children in reverse order so they are visited in left-to-right order.
                for child in node.children().iter().rev() {
                    self.stack.push((child, depth + 1, false));
                }
            }
        }
        None
    }
}
