use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode, NodeId};

/// An iterator that traverses an `ArenaAst` tree in postorder (depth-first).
///
/// The traversal visits all children of a node before visiting the node itself.
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::semantic::arena::{ArenaAst, PostorderIter};
///
/// let mut arena = ArenaAst::new();
/// let root = arena.add_node(/* kind */ .., /* span */ .., vec![]);
///
/// let child1 = arena.add_node(/* kind */ .., /* span */ .., vec![]);
/// let child2 = arena.add_node(/* kind */ .., /* span */ .., vec![]);
///
/// arena.get_node_mut(root).unwrap().add_child(child1);
/// arena.get_node_mut(root).unwrap().add_child(child2);
///
/// let iter = PostorderIter::new(&arena, root);
/// for node in iter {
///     println!("{:?}", node.kind());
/// }
/// ```
///
/// # Implementation details
///
/// This iterator uses an explicit stack to avoid recursion.
/// Each stack element stores a tuple `(NodeId, next_child_index)`.
/// `next_child_index` tracks which child to visit next for a node.
///
/// On each call to `next()`, the iterator:
/// - Checks if the next child index is within bounds:
///     - If yes, it pushes back the current node with the next child index incremented,
///       then pushes the child node with child index `0` (start traversal).
/// - Otherwise, it yields the node itself after all children are visited.
///
/// This design is efficient in time and memory, and avoids stack overflow.
///
/// # Note
///
/// `NodeId` is a wrapper around a numeric index identifying nodes in the arena.
/// Children are stored as a `Vec<NodeId>`.
pub struct PostorderIter<'a> {
    arena: &'a ArenaAst,
    stack: Vec<(NodeId, usize)>, // (node_id, next_child_index)
}

impl<'a> PostorderIter<'a> {
    /// Creates a new postorder iterator starting from the given `root` node.
    ///
    /// # Arguments
    ///
    /// * `arena` - Reference to the arena containing the AST nodes.
    /// * `root` - The `NodeId` of the root node to start traversal from.
    ///
    /// # Returns
    ///
    /// A `PostorderIter` ready to traverse the subtree rooted at `root`.
    pub fn new(arena: &'a ArenaAst, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![(root, 0)],
        }
    }
}

impl<'a> Iterator for PostorderIter<'a> {
    type Item = &'a ArenaAstNode;

    /// Advances the iterator and returns the next node in postorder.
    ///
    /// Returns `None` when the entire subtree has been traversed.
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((node_id, child_idx)) = self.stack.pop() {
            let node = self.arena.get_node(node_id)?;
            if child_idx < node.children().len() {
                // Push the current node back with incremented child index,
                // so that on the next iteration, we visit the next child.
                self.stack.push((node_id, child_idx + 1));
                // Push the child node to traverse its subtree first.
                self.stack.push((node.children()[child_idx], 0));
            } else {
                // All children have been visited, yield this node.
                return Some(node);
            }
        }
        None
    }
}
