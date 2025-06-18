use crate::aiplan4rust::syntax::int_ast::IntAstNode;

/// An iterator that traverses an `IntAstNode` tree in preorder.
///
/// Preorder traversal visits the current node before its children.
/// This iterator yields each node along with its depth in the tree,
/// where the root node has depth 0.
///
/// # Example
///
/// ```
/// let root: &IntAstNode = ...;
/// let iter = PreorderIter::new(root);
/// for (node, depth) in iter {
///     println!("{}Node: {:?}", "  ".repeat(depth), node);
/// }
/// ```
pub struct PreorderIter<'a> {
    /// Stack to manage traversal state.
    /// Each element is a tuple of a node reference and its depth.
    stack: Vec<(&'a IntAstNode, usize)>,
}

impl<'a> PreorderIter<'a> {
    /// Creates a new `PreorderIter` starting at the given root node.
    ///
    /// # Arguments
    ///
    /// * `root` - A reference to the root node of the AST to traverse.
    ///
    /// # Returns
    ///
    /// A new instance of `PreorderIter`.
    pub fn new(root: &'a IntAstNode) -> Self {
        Self { stack: vec![(root, 0)] }
    }
}

impl<'a> Iterator for PreorderIter<'a> {
    type Item = (&'a IntAstNode, usize);

    /// Advances the iterator and returns the next node and its depth.
    ///
    /// The traversal order is preorder: the node itself is returned before its children.
    ///
    /// # Returns
    ///
    /// * `Some((&IntAstNode, usize))` - the next node and its depth in the tree.
    /// * `None` - when all nodes have been visited.
    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop().map(|(node, depth)| {
            // Push children to the stack in reverse order so that
            // they are processed in original left-to-right order.
            for child in node.children().iter().rev() {
                self.stack.push((child, depth + 1));
            }
            (node, depth)
        })
    }
}
