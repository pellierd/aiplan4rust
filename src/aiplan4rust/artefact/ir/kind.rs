use std::fmt;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};

/// Type sémantique de l’objet
#[derive(Debug, Clone, Copy, PartialEq, Eq,  Serialize, Deserialize)]
pub enum IRKind {
    ParsedDomain,
    ParsedProblem,
    LiftedProblem,
}

impl Display for IRKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            IRKind::ParsedDomain => "Domain",
            IRKind::ParsedProblem => "Problem",
            IRKind::LiftedProblem => "Lifted Problem",
        };
        write!(f, "{}", s)
    }
}
