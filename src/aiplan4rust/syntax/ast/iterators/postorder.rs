use crate::aiplan4rust::syntax::ast::AstNode;

/// Iterator for traversing an [`AstNode`] tree in **post-order** (depth-first),
/// yielding each node along with its depth.
///
/// In post-order traversal, all children of a node are visited before the node itself.
/// This is useful for tasks like expr evaluation or resource cleanup.
///
/// # Example
///
/// ```rust
/// use aiplan4rust::syntax::ast::{AstNode, PostorderIter};
///
/// let root: &AstNode = get_ast_root(); // assume this returns your AST root node
/// let iter = PostorderIter::new(root);
///
/// for (node, depth) in iter {
///     println!("{}- {:?}", "  ".repeat(depth), node.kind());
/// }
/// ```
///
/// # Internals
///
/// The iterator uses a manual stack to simulate recursion. Each entry is a tuple:
/// - `&AstNode`: the node reference,
/// - `usize`: the depth in the tree,
/// - `bool`: whether the node has already been visited after processing children.
///
/// Children are pushed in reverse order to preserve left-to-right traversal.
pub struct PostorderIter<'a> {
    stack: Vec<(&'a AstNode, usize, bool)>,
}

impl<'a> PostorderIter<'a> {
    /// Creates a new `PostorderIter` starting at the given root node.
    pub fn new(root: &'a AstNode) -> Self {
        Self {
            stack: vec![(root, 0, false)],
        }
    }
}

impl<'a> Iterator for PostorderIter<'a> {
    type Item = (&'a AstNode, usize);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node, depth, visited)) = self.stack.pop() {
            if visited {
                return Some((node, depth));
            } else {
                self.stack.push((node, depth, true));
                for child in node.children().iter().rev() {
                    self.stack.push((child, depth + 1, false));
                }
            }
        }
        None
    }
}
