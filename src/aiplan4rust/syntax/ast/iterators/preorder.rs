use crate::aiplan4rust::syntax::ast::AstNode;

/// Iterator for traversing an [`AstNode`] tree in **pre-order** (depth-first),
/// yielding each node along with its depth.
///
/// In pre-order traversal, the current node is visited *before* its children.
/// This is useful for printing, structural validation, or transforming trees top-down.
///
/// # Example
///
/// ```rust
/// use aiplan4rust::syntax::ast::{AstNode, PreorderIter};
///
/// let root: &AstNode = get_ast_root(); // assume this returns your AST root
/// let iter = PreorderIter::new(root);
///
/// for (node, depth) in iter {
///     println!("{}- {:?}", "  ".repeat(depth), node.kind());
/// }
/// ```
///
/// # Internals
///
/// The iterator maintains a manual stack where each entry is:
/// - `&AstNode`: the node reference,
/// - `usize`: the depth of the node.
///
/// Children are pushed in reverse order to ensure left-to-right traversal.
pub struct PreorderIter<'a> {
    stack: Vec<(&'a AstNode, usize)>,
}

impl<'a> PreorderIter<'a> {
    /// Creates a new `PreorderIter` starting from the given root node.
    pub fn new(root: &'a AstNode) -> Self {
        Self {
            stack: vec![(root, 0)],
        }
    }
}

impl<'a> Iterator for PreorderIter<'a> {
    type Item = (&'a AstNode, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop().map(|(node, depth)| {
            for child in node.children().iter().rev() {
                self.stack.push((child, depth + 1));
            }
            (node, depth)
        })
    }
}
