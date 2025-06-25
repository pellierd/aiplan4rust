use std::collections::HashMap;
use crate::aiplan4rust::arena::{Arena, NodeId};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::symbol::SymbolRef;
use crate::aiplan4rust::syntax::elements::Ident;

/// Trait générique pour tout type de nœud utilisé dans une `Arena`.
pub trait NodeTrait {
    fn parent(&self) -> Option<NodeId>;
    fn children(&self) -> &[NodeId];

    fn set_parent(&mut self, parent: Option<NodeId>);
    fn add_child(&mut self, child: NodeId);

    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>);

    /// Returns `true` if this node has no children.
    ///
    /// # Returns
    /// * `true` if the node is a leaf.
    /// * `false` otherwise.
    fn is_leaf(&self) -> bool {
        self.children().is_empty()
    }

    /// Returns the number of direct children of this node.
    ///
    /// # Returns
    /// * `usize` - The number of immediate child nodes.
    fn arity(&self) -> usize {
        self.children().len()
    }

    fn try_symbol_ref(&self, arena: &Arena<Self>) -> Result<SymbolRef, ParserInternalError>
    where
        Self: Sized;

}
