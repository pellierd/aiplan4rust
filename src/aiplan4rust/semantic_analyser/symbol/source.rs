use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
pub enum Source {
    #[default]
    Domain,
    Problem,
    Unknown,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Domain => write!(f, "Domain"),
            Source::Problem => write!(f, "Problem"),
            Source::Unknown => write!(f, "Unknown"),
        }
    }
}
