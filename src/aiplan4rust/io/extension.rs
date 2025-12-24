use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Extension {
    // Raw input
    Pddl,
    Hddl,

    // Pipeline artifacts
    Parsed,   // .prs
    Lifted,  // .lift
    Ground,  // .grd

}

impl Extension {
    pub fn as_str(&self) -> &'static str {
        match self {
            Extension::Pddl => "pddl",
            Extension::Hddl => "hddl",
            Extension::Parsed => "prs",
            Extension::Lifted => "lift",
            Extension::Ground => "grd",
        }
    }
}

// Implement Display for pretty printing
impl fmt::Display for Extension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
