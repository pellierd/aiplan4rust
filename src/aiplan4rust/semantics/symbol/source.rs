use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
pub enum Source {
    #[default]
    Domain,
    Problem,
    Unknown,
}

impl fmt::Display for crate::aiplan4rust::semantics::symbol::Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            crate::aiplan4rust::semantics::symbol::Source::Domain => write!(f, "Domain"),
            crate::aiplan4rust::semantics::symbol::Source::Problem => write!(f, "Problem"),
            crate::aiplan4rust::semantics::symbol::Source::Unknown => write!(f, "Unknown"),
        }
    }
}
