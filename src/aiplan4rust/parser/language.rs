use serde::Deserialize;
use serde::Serialize;
use std::str::FromStr;

/// An enum representing different types of PDDL and HDDL expressions.
///
/// The `Language` enum is used to specify which type of planning language is being
/// used in the context of parsing. It currently supports two variants:
/// PDDL (Planning Domain Definition Language) and HDDL (Hierarchical Domain Definition Language).
/// These variants help determine the syntax and semantics of the expressions being parsed.
///
/// # Variants
/// - `PDDL`: Represents the Planning Domain Definition Language (PDDL), a widely used language for
///   defining planning problems and domains.
/// - `HDDL`: Represents the Hierarchical Domain Definition Language (HDDL), an extension of PDDL
///   that incorporates hierarchical structures for domain and problem representations.
///
/// # Example
/// ```rust
/// let language = Language::PDDL;
/// ```
///
/// # Notes
/// - This enum is designed to support multiple types of planning languages. As the `aiplan4rust`
///   library evolves, additional languages or variants may be added to accommodate new planning
///   languages or extensions.

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    PDDL,
    HDDL,
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pddl" => Ok(Language::PDDL),
            "hddl" => Ok(Language::HDDL),
            _ => Err(format!("Invalid language: {}", s)),
        }
    }
}
