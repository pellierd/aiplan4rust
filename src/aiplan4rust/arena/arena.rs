use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::arena::Node;
use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::arena::iterators::{PostorderIter, PostorderIterWithIndex, PreorderIterWithIndex};
use crate::aiplan4rust::arena::iterators::PreorderIter;
use crate::aiplan4rust::arena::node_ref::{NodeRef, NodeRefMut};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::arena::ArenaAstNode;
use crate::aiplan4rust::semantic::symbol::SymbolRef;
use crate::aiplan4rust::syntax::ast::{Ast, AstContent, AstNode};
use crate::aiplan4rust::syntax::elements::Ident;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Arena<T: Node> {
    nodes: Vec<T>,
}

impl<T: Node> Arena<T> {
    pub fn new() -> Self {
        Arena { nodes: Vec::new() }
    }

    /// Ajoute un élément `T` dans l'arène, retourne son NodeId.
    pub fn add(&mut self, mut content: T, parent_id: Option<NodeId>) -> NodeId {
        let id = NodeId::new(self.nodes.len());
        content.set_parent(parent_id); // on définit le parent
        self.nodes.push(content);

        if let Some(pid) = parent_id {
            self.nodes[pid.as_usize()].add_child(id); // le parent enregistre ce nouveau nœud comme enfant
        }

        id
    }

    pub fn root_node(&self) -> Option<&T> {
        self.get_node(NodeId::ROOT_ID)
    }

    /// Retourne une référence au nœud racine encapsulée dans un `NodeRef`.
    ///
    /// # Returns
    /// * `Some(NodeRef)` si la racine existe.
    /// * `None` sinon.
    pub fn root_node_ref(&self) -> Option<NodeRef<'_, T>> {
        self.get_node_ref(NodeId::ROOT_ID)
    }

    /// Récupère une référence immuable au contenu par NodeId.
    pub fn get_node(&self, id: NodeId) -> Option<&T> {
        self.nodes.get(id.as_usize())
    }

    /// Récupère une référence immuable au nœud avec son identifiant.
    ///
    /// Retourne un `NodeRef` qui encapsule l’`Id` et une référence au nœud.
    ///
    /// # Arguments
    ///
    /// * `id` - L’identifiant du nœud à récupérer.
    ///
    /// # Returns
    ///
    /// * `Some(NodeRef)` si le nœud existe,
    /// * `None` sinon.
    pub fn get_node_ref(&self, id: NodeId) -> Option<NodeRef<'_, T>> {
        self.nodes.get(id.as_usize()).map(|node| NodeRef::new(id, node))
    }

    /// Récupère une référence mutable au contenu par NodeId.
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut T> {
        self.nodes.get_mut(id.as_usize())
    }

    /// Récupère un `NodeRefMut` (Id + &mut T) ou `None` si inexistant.
    pub fn get_ref_mut(&mut self, id: NodeId) -> Option<NodeRefMut<'_, T>> {
        self.get_node_mut(id).map(|node| NodeRefMut::new(id, node))
    }

    pub fn try_node(&self, id: NodeId) -> Result<&T, ParserInternalError> {
        self.get_node(id).ok_or_else(|| {
            ParserInternalError::new(format!("Node with id {} not found", id))
        })
    }

    pub fn try_node_ref(&self, id: NodeId) -> Result<NodeRef<'_, T>, ParserInternalError> {
        let node = self.try_node(id)?;
        Ok(NodeRef::new(id, node))
    }

    pub fn try_node_mut(&mut self, id: NodeId) -> Result<&mut T, ParserInternalError> {
        self.get_node_mut(id).ok_or_else(|| {
            ParserInternalError::new(format!("Node with id {} not found", id))
        })
    }

    pub fn try_node_ref_mut(&mut self, id: NodeId) -> Result<NodeRefMut<'_, T>, ParserInternalError> {
        let node = self.try_node_mut(id)?;
        Ok(NodeRefMut::new(id, node))
    }

    pub fn try_symbol_ref(&self, id: NodeId) -> Result<SymbolRef, ParserInternalError> {
        let node = self.try_node(id)?;
        node.try_symbol_ref(self)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Retourne un itérateur en profondeur (pré-ordre) depuis la racine.
    pub fn preorder(&self) -> PreorderIter<'_, T> {
        PreorderIter::new(self, NodeId::ROOT_ID)
    }

    /// Retourne un itérateur pré-ordre à partir d’un nœud spécifique.
    pub fn preorder_from(&self, root: NodeId) -> PreorderIter<'_, T> {
        PreorderIter::new(self, root)
    }

    /// Retourne un itérateur pré-ordre avec index.
    pub fn preorder_with_index(&self) -> PreorderIterWithIndex<'_, T> {
        PreorderIterWithIndex::new(self, NodeId::ROOT_ID)
    }

    /// Retourne un itérateur en profondeur (post-ordre) depuis la racine.
    pub fn postorder(&self) -> PostorderIter<'_, T> {
        PostorderIter::new(self, NodeId::ROOT_ID)
    }

    /// Retourne un itérateur post-ordre à partir d’un nœud spécifique.
    pub fn postorder_from(&self, root: NodeId) -> PostorderIter<'_, T> {
        PostorderIter::new(self, root)
    }

    /// Retourne un itérateur post-ordre avec index.
    pub fn postorder_with_index(&self) -> PostorderIterWithIndex<'_, T> {
        PostorderIterWithIndex::new(self, NodeId::ROOT_ID)
    }

    /// Remap les idents en partant de la racine (ROOT_ID).
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        self.remap_idents_from(NodeId::ROOT_ID, map);
    }

    /// Remap les idents en partant d'un nœud donné (sous-arbre).
    pub fn remap_idents_from(&mut self, id: NodeId, map: &HashMap<Ident, Ident>) {
        let mut stack = vec![id];

        while let Some(current_id) = stack.pop() {
            if let Some(node) = self.get_node_mut(current_id) {
                node.remap_idents(map);

                for &child_id in node.children() {
                    stack.push(child_id);
                }
            }
        }
    }

    pub fn size(&self, root: NodeId) -> usize {
        let mut count = 0;
        let mut stack = vec![root];

        while let Some(node_id) = stack.pop() {
            count += 1;
            if let Some(node) = self.get_node(node_id) {
                for &child_id in node.children() {
                    stack.push(child_id);
                }
            }
        }

        count
    }

    pub fn depth(&self, root: NodeId) -> usize {
        let mut max_depth = 0;
        let mut stack = vec![(root, 1)];

        while let Some((node_id, depth)) = stack.pop() {
            if depth > max_depth {
                max_depth = depth;
            }
            if let Some(node) = self.get_node(node_id) {
                for &child_id in node.children() {
                    stack.push((child_id, depth + 1));
                }
            }
        }

        max_depth
    }
}

/*impl Arena<ArenaAstNode> {
    /// Construit une arène à partir d'un AST standard (`Ast`) en itératif.
    ///
    /// # Exemple
    ///
    /// ```
    /// let arena = Arena::<ArenaAstNode>::from_ast(&ast);
    /// ```
    pub fn from_ast(ast: &Ast) -> Self {
        let mut arena = Arena::new();
        let root = ast.root();
        Self::add_iterative(&mut arena, root, None);
        arena
    }

    /// Ajoute récursivement les nœuds dans l'arène en conservant les parents.
    fn add_iterative(arena: &mut Arena<ArenaAstNode>, root: &AstNode, parent_id: Option<NodeId>) -> NodeId {
        use std::collections::HashMap;

        let mut stack = vec![(root, parent_id)];
        let mut node_ids = HashMap::<*const AstNode, NodeId>::new();

        while let Some((node, parent)) = stack.pop() {
            let arena_node = ArenaAstNode::from_ast_node(node); // convertit AstNode -> ArenaAstNode
            let node_id = arena.add(arena_node, parent);
            node_ids.insert(node as *const _, node_id);

            for child in node.children().iter().rev() {
                stack.push((child.as_ref(), Some(node_id)));
            }
        }

        node_ids[&(root as *const _)]
    }
}*/

/*impl<T> fmt::Display for Arena<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn fmt_node<T: fmt::Display>(arena: &Arena<T>, id: NodeId, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
            if let Some(node) = arena.get_node(id) {
                writeln!(f, "{}{}", "  ".repeat(depth), node.content)?;
                for &child in &node.children {
                    fmt_node(arena, child, f, depth + 1)?;
                }
            }
            Ok(())
        }
        if !self.nodes.is_empty() {
            fmt_node(self, NodeId:ROOT_NODE_ID, f, 0)?;
        }
        Ok(())
    }
}*/
