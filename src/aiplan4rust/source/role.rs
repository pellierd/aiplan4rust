use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::fmt;

/// Constants for source role identifiers.
pub const DOMAIN_ROLE: &str = "domain";
pub const PROBLEM_ROLE: &str = "problem";

/// Semantic role of a planning source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceRole {
    Domain,
    Problem,
}

impl Default for SourceRole {
    fn default() -> Self {
        SourceRole::Domain
    }
}

impl fmt::Display for SourceRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SourceRole::Domain => DOMAIN_ROLE,
            SourceRole::Problem => PROBLEM_ROLE,
        };
        write!(f, "{}", s)
    }
}

impl FromStr for SourceRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            DOMAIN_ROLE => Ok(SourceRole::Domain),
            PROBLEM_ROLE => Ok(SourceRole::Problem),
            _ => Err(format!("Invalid source role: '{}'", s)),
        }
    }
}
