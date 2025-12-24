use std::fmt;

/// Langage du contenu brut
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Language {
    PDDL,
    HDDL,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::PDDL => write!(f, "PDDL"),
            Language::HDDL => write!(f, "HDDL"),
        }
    }
}
