use crate::aiplan4rust::arena::{Arena, NodeId};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::frontend::ParserInternalError;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Enum qui identifie clairement le scope d’origine,
/// soit un Domain, soit un Problem, chacun avec sa stack de NodeId.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScopeOrigin {
    Domain(Vec<NodeId>),
    Problem(Vec<NodeId>),
}

impl ScopeOrigin {
    /// Crée un nouveau scope Domain, éventuellement en héritant d’un parent
    pub fn new_domain(ast: NodeId, parent: Option<&ScopeOrigin>) -> Self {
        let mut stack = Vec::new();
        if let Some(parent_scope) = parent {
            if let ScopeOrigin::Domain(ref parent_stack) = parent_scope {
                stack.extend(parent_stack.iter().cloned());
            }
            // Sinon pas d'héritage entre Domain et Problem
        }
        stack.push(ast);
        ScopeOrigin::Domain(stack)
    }

    /// Crée un nouveau scope Problem, éventuellement en héritant d’un parent
    pub fn new_problem(ast: NodeId, parent: Option<&ScopeOrigin>) -> Self {
        let mut stack = Vec::new();
        if let Some(parent_scope) = parent {
            if let ScopeOrigin::Problem(ref parent_stack) = parent_scope {
                stack.extend(parent_stack.iter().cloned());
            }
            // Sinon pas d'héritage entre Problem et Domain
        }
        stack.push(ast);
        ScopeOrigin::Problem(stack)
    }

    /// Récupère la stack des NodeId quel que soit le type
    pub fn stack(&self) -> &[NodeId] {
        match self {
            ScopeOrigin::Domain(stack) => stack.as_slice(),
            ScopeOrigin::Problem(stack) => stack.as_slice(),
        }
    }

    /// Vérifie si le scope commence par un autre scope (prefix matching)
    /// Comparaison seulement si même variante Domain vs Domain ou Problem vs Problem
    pub fn starts_with(&self, prefix: &ScopeOrigin) -> bool {
        match (self, prefix) {
            (ScopeOrigin::Domain(s), ScopeOrigin::Domain(p)) => s.starts_with(p),
            (ScopeOrigin::Problem(s), ScopeOrigin::Problem(p)) => s.starts_with(p),
            // Domain et Problem sont incompatibles
            _ => false,
        }
    }

    /// Itérateur sur les NodeId du scope
    pub fn iter(&self) -> impl Iterator<Item = &NodeId> {
        self.stack().iter()
    }

    /// Vérifie si un noeud d'un certain AstKind est présent dans le scope
    pub fn contains_node_of_kind(
        &self,
        kind: AstKind,
        ast: &Arena<AstNode>,
    ) -> Result<bool, ParserInternalError> {
        for &id in self.iter() {
            let node = ast.try_node(id)?;
            if node.kind() == kind {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl fmt::Display for ScopeOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        let mut first = true;
        for node_id in self.stack() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}", node_id.as_usize())?;
            first = false;
        }
        write!(f, "]")
    }
}
