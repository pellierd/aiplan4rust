use crate::aiplan4rust::arena::{Arena, NodeId, ArenaNode};

/// A postorder iterator over nodes in an `Arena`, yielding `(NodeId, &T)` pairs.
///
/// This iterator traverses the arena in postorder, meaning it visits all children of a syntax
/// before the syntax itself. Each iteration returns the unique syntax identifier along with
/// a reference to the syntax.
///
/// The traversal preserves left-to-right order by pushing children onto the stack in reverse.
///
/// # Example
///
/// ```rust
/// let iter = PostorderIterWithIndex::new(&arena, root_id);
/// for (id, syntax) in iter {
///     // Process syntax with its id
/// }
/// ```
pub struct PostorderIterWithIndex<'a, T: ArenaNode> {
    arena: &'a Arena<T>,
    stack: Vec<(NodeId, bool)>, // (syntax id, children visited flag)
}

impl<'a, T: ArenaNode> PostorderIterWithIndex<'a, T> {
    /// Creates a new postorder iterator starting from the specified root syntax.
    ///
    /// # Parameters
    ///
    /// * `arena` - Reference to the arena containing the arena nodes.
    /// * `root` - The root syntax ID from which to start traversal.
    ///
    /// # Returns
    ///
    /// A `PostorderIterWithIndex` ready to traverse the arena in postorder.
    pub fn new(arena: &'a Arena<T>, root: NodeId) -> Self {
        Self {
            arena,
            stack: vec![(root, false)],
        }
    }

    /// Creates an empty postorder iterator with no nodes.
    pub fn empty(arena: &'a Arena<T>) -> Self {
        Self {
            arena,
            stack: Vec::new(),
        }
    }
}

impl<'a, T: ArenaNode> Iterator for PostorderIterWithIndex<'a, T> {
    type Item = (NodeId, &'a T);

    /// Returns the next syntax in postorder traversal along with its ID.
    ///
    /// The traversal visits all children of a syntax before the syntax itself.
    ///
    /// Returns `None` when traversal is complete.
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&(id, visited)) = self.stack.last() {
            if !visited {
                // Mark this syntax as visited to indicate children will be processed
                if let Some(top) = self.stack.last_mut() {
                    top.1 = true;
                }

                // Push children in reverse order to traverse them left-to-right
                if let Some(node) = self.arena.get_node(id) {
                    for &child_id in node.children().iter().rev() {
                        self.stack.push((child_id, false));
                    }
                }
            } else {
                // All children visited; yield this syntax
                self.stack.pop();
                if let Some(node) = self.arena.get_node(id) {
                    return Some((id, node));
                }
            }
        }
        None
    }
}
