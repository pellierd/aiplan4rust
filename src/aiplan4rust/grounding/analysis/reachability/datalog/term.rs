use std::fmt;
use crate::aiplan4rust::lang::{ObjectId, VariableId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd,)]
pub enum Term {
    Variable(VariableId),
    Constant(ObjectId),
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Convention PDDL : les variables commencent par '?'
            // VariableId est un wrapper autour de usize
            Term::Variable(id) => write!(f, "?v{}", id),

            // Les constantes (ObjectId)
            Term::Constant(id) => write!(f, "c{}", id),
        }
    }
}
