use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::arena::node::Node;
use crate::aiplan4rust::semantic::arena::iterators::PostorderIter;
use crate::aiplan4rust::semantic::arena::iterators::PostorderIterWithIndex;
use crate::aiplan4rust::semantic::arena::iterators::PreorderIter;
use crate::aiplan4rust::semantic::arena::iterators::PreorderIterWithIndex;
use crate::aiplan4rust::syntax::{AstKind, AstNode, Span};
use crate::aiplan4rust::syntax::ast::Ast;

/// Arena is a data structure that stores AST nodes in a contiguous vector.
/// Each node keeps track of its children and its parent by index.
///
/// This structure allows efficient traversal and manipulation of the AST
/// without the need for heap allocations per node.
///
/// Nodes are identified by their index in the `nodes` vector.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Arena {
    nodes: Vec<Node>,
}

impl Arena {
    /// Creates a new, empty Arena.
    ///
    /// # Examples
    ///
    /// ```
    /// let arena = Arena::new();
    /// ```
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Adds a node with a given parent.
    ///
    /// # Parameters
    ///
    /// - `kind`: The kind of AST node to add.
    /// - `span`: The source code span associated with the node.
    /// - `parent_id`: The index of the parent node, or `None` if this node is the root.
    ///
    /// # Returns
    ///
    /// Returns the index of the newly added node in the arena.
    ///
    /// # Behavior
    ///
    /// If `parent_id` is `Some`, this node is added as a child of the parent node,
    /// and its parent link is set accordingly.
    ///
    /// If `parent_id` is `None`, this node is treated as a root node with no parent.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut arena = Arena::new();
    /// let root_id = arena.add(root_kind, root_span, None);
    /// let child_id = arena.add(child_kind, child_span, Some(root_id));
    /// ```
    pub fn add(&mut self, kind: AstKind, span: Span, parent_id: Option<usize>) -> usize {
        let id = self.nodes.len();
        let node = Node::new(kind, span, parent_id);
        self.nodes.push(node);
        if let Some(pid) = parent_id {
            self.nodes[pid].add_child(id);
        }
        id
    }

    /// Returns an immutable reference to the node at the given index, if it exists.
    ///
    /// # Parameters
    ///
    /// - `id`: The index of the node to retrieve.
    ///
    /// # Returns
    ///
    /// `Some(&Node)` if the node exists, or `None` otherwise.
    pub fn get_node(&self, id: usize) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Returns an immutable reference to the parent node of the node at the given index, if it exists.
    ///
    /// # Parameters
    ///
    /// - `id`: The index of the node whose parent is to be retrieved.
    ///
    /// # Returns
    ///
    /// `Some(&Node)` if the parent node exists, or `None` if the node has no parent or the id is invalid.
    pub fn get_parent(&self, id: usize) -> Option<&Node> {
        self.nodes.get(id)
            .and_then(|node| node.parent())   // Assuming `parent` is an Option<usize>
            .and_then(|parent_id| self.nodes.get(parent_id))
    }

    /// Attempts to retrieve the symbol associated with the node at the given index.
    ///
    /// This method returns:
    /// - `Ok(Some(&String))` if a symbol is found associated with the node.
    /// - `Ok(None)` if the node does not have an associated symbol (this is not considered an error).
    /// - `Err(ParserInternalError)` if the node does not exist or is malformed (e.g., a `FunctionTerm` or `AtomicFormula` node
    ///   without children, or a missing child node).
    ///
    /// The method first tries to get the symbol directly from the node's kind. If none is found,
    /// and the node is a compound type like `FunctionTerm` or `AtomicFormula`, it attempts to derive
    /// the symbol from the first child node recursively.
    ///
    /// # Parameters
    /// - `id`: The index of the node in the arena.
    ///
    /// # Returns
    /// - `Result<Option<&String>, ParserInternalError>`:
    ///    - `Ok(Some(symbol))` if a symbol was found.
    ///    - `Ok(None)` if no symbol is associated with the node.
    ///    - `Err(ParserInternalError)` if the node or its first child is missing or malformed.
    pub fn get_symbol(&self, id: usize) -> Result<Option<&String>, ParserInternalError> {
        let node = self.get_node(id).ok_or_else(|| ParserInternalError::new("Node not found".to_string()))?;
        if let Some(sym) = node.kind().get_symbol() {
            return Ok(Some(sym));
        }
        match &node.kind() {
            AstKind::FunctionTerm | AstKind::AtomicFormula => {
                let child_idx = node.children().first().ok_or_else(|| {
                    ParserInternalError::new("No children found for FunctionTerm or AtomicFormula".to_string())
                })?;
                let child = self.get_node(*child_idx).ok_or_else(|| {
                    ParserInternalError::new(format!("Child node {} not found", child_idx))
                })?;
                Ok(child.kind().get_symbol())
            }
            _ => Ok(None),
        }
    }


    /// Returns a mutable reference to the node at the given index, if it exists.
    ///
    /// # Parameters
    ///
    /// - `id`: The index of the node to retrieve.
    ///
    /// # Returns
    ///
    /// `Some(&mut Node)` if the node exists, or `None` otherwise.
    pub fn get_mut(&mut self, id: usize) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    /// Returns the total number of nodes stored in the arena.
    ///
    /// This represents the size of the AST subtree (or forest) currently
    /// held within the arena.
    ///
    /// # Returns
    ///
    /// The number of nodes in the arena.
    ///
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Constructs an Arena from a given AST.
    ///
    /// Traverses the AST iteratively and populates the arena with nodes.
    /// The root node is added with no parent.
    ///
    /// # Parameters
    ///
    /// - `ast`: The AST from which to build the arena.
    ///
    /// # Returns
    ///
    /// A fully populated `Arena` representing the AST.
    ///
    /// # Examples
    ///
    /// ```
    /// let arena = Arena::from_ast(&ast);
    /// ```
    pub fn from_ast(ast: &Ast) -> Self {
        let mut arena = Arena::new();
        let root = ast.root();
        Self::add_iterative(&mut arena, root, None);
        arena
    }

    /// Iteratively adds nodes from the given AST node into the arena.
    ///
    /// This function uses a stack to avoid recursion. It maintains a mapping
    /// from AST node pointers to arena indices to track correspondence.
    ///
    /// # Parameters
    ///
    /// - `arena`: The arena to populate.
    /// - `root`: The root AST node to start adding from.
    /// - `parent_id`: The optional parent node index in the arena. `None` indicates root.
    ///
    /// # Returns
    ///
    /// Returns the index of the root node added to the arena.
    fn add_iterative(arena: &mut Arena, root: &AstNode, parent_id: Option<usize>) -> usize {
        use std::collections::HashMap;

        let mut stack = vec![(root, parent_id)];
        let mut node_ids = HashMap::<*const AstNode, usize>::new();

        while let Some((node, parent)) = stack.pop() {
            let node_id = arena.add(node.kind().clone(), node.span().clone(), parent);
            node_ids.insert(node as *const _, node_id);

            // Push children onto the stack with current node as their parent,
            // iterating in reverse order to maintain original child order.
            for child_box in node.children().iter().rev() {
                stack.push((child_box.as_ref(), Some(node_id)));
            }
        }

        // Return the arena index corresponding to the root AST node
        node_ids[&(root as *const _)]
    }

    /// Returns an iterator over the tree in preorder (depth-first).
    ///
    /// Traverses the tree starting from the root node (index 0),
    /// visiting each node before its children (left to right).
    ///
    /// # Example
    /// ```rust
    /// for node in arena.preorder() {
    ///     println!("{:?}", node);
    /// }
    /// ```
    pub fn preorder(&self) -> PreorderIter<'_> {
        PreorderIter::new(self, 0)
    }

    /// Returns a preorder (depth-first) iterator starting from the specified node index.
    ///
    /// This iterator traverses the subtree rooted at `root`, visiting each node
    /// before its children (left to right).
    ///
    /// This allows traversing any subtree within the arena, not seulement la racine.
    ///
    /// # Parameters
    /// - `root`: The index of the node from which to start the traversal.
    ///
    /// # Returns
    /// A `PreorderIter` that yields references to nodes in preorder.
    ///
    /// # Examples
    /// ```rust
    /// let arena = Arena::from_ast(&ast);
    /// let start = 5; // Index of the node to start traversal from
    /// for node in arena.preorder_from(start) {
    ///     println!("{:?}", node.kind());
    /// }
    /// ```
    pub fn preorder_from(&self, root: usize) -> PreorderIter<'_> {
        PreorderIter::new(self, root)
    }

    /// Returns an iterator over the tree in preorder (depth-first),
    /// yielding each node’s index and a reference to the node.
    ///
    /// Traverses the tree starting from the root node (index 0),
    /// visiting each node before its children (left to right).
    ///
    /// # Example
    /// ```rust
    /// for (idx, node) in arena.preorder_with_index() {
    ///     println!("Node index: {}, kind: {:?}", idx, node.kind());
    /// }
    /// ```
    pub fn preorder_with_index(&self) -> PreorderIterWithIndex<'_> {
        PreorderIterWithIndex::new(self, 0)
    }

    /// Returns an iterator over the tree in postorder (depth-first).
    ///
    /// Traverses the tree starting from the root node (index 0),
    /// visiting each node’s children before the node itself (left to right).
    ///
    /// # Example
    /// ```rust
    /// for node in arena.postorder() {
    ///     println!("{:?}", node);
    /// }
    /// ```
    pub fn postorder(&self) -> PostorderIter<'_> {
        PostorderIter::new(self, 0)
    }

    /// Returns a postorder (depth-first) iterator starting from the specified node index.
    ///
    /// This iterator traverses the subtree rooted at `root`, visiting each node’s
    /// children before the node itself (left to right).
    ///
    /// Useful for traversing any subtree within the arena, not seulement la racine.
    ///
    /// # Parameters
    /// - `root`: The index of the node from which to start the traversal.
    ///
    /// # Returns
    /// A `PostorderIter` that yields references to nodes in postorder.
    ///
    /// # Examples
    /// ```rust
    /// let arena = Arena::from_ast(&ast);
    /// let start = 3; // Index of the node to start traversal from
    /// for node in arena.postorder_from(start) {
    ///     println!("{:?}", node.kind());
    /// }
    /// ```
    pub fn postorder_from(&self, root: usize) -> PostorderIter<'_> {
        PostorderIter::new(self, root)
    }

    /// Returns an iterator over the tree in postorder (depth-first),
    /// yielding each node’s index and a reference to the node.
    ///
    /// Traverses the tree starting from the root node (index 0),
    /// visiting each node’s children before the node itself (left to right).
    ///
    /// # Example
    /// ```rust
    /// for (idx, node) in arena.postorder_with_index() {
    ///     println!("Node index: {}, kind: {:?}", idx, node.kind());
    /// }
    /// ```
    pub fn postorder_with_index(&self) -> PostorderIterWithIndex<'_> {
        PostorderIterWithIndex::new(self, 0)
    }
}

impl fmt::Display for Arena {
    /// Formats the entire AST stored in the `Arena` as a human-readable,
    /// indented tree structure.
    ///
    /// Each node is displayed on its own line, prefixed by its index in the arena
    /// and indented according to its depth in the tree. The node itself is
    /// formatted using its own `Display` implementation, which shows the
    /// node's kind, span, parent index, and list of child indices.
    ///
    /// # Panics
    ///
    /// This implementation should never panic unless the underlying arena data is corrupted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, node) in self.preorder_with_index() {
            let mut depth = 0;
            let mut current = node.parent();
            while let Some(p) = current {
                depth += 1;
                current = self.get_node(p).and_then(|n| n.parent());
            }

            let indent = "  ".repeat(depth);
            writeln!(f, "{}[{}] {}", indent, idx, node)?;
        }
        Ok(())
    }
}
