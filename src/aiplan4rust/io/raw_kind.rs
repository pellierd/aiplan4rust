use std::fmt;

/// Represents the type of a raw planning file.
///
/// `RawKind` is used to distinguish between domain and problem files
/// in planning formalisms such as PDDL or HDDL.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RawKind {
    /// Represents a domain file, e.g., containing types, predicates, and actions.
    Domain,

    /// Represents a problem file, e.g., containing objects, initial state, and goals.
    Problem,
}

impl fmt::Display for RawKind {
    /// Formats the `RawKind` as a human-readable string.
    ///
    /// # Examples
    ///
    /// ```
    /// use aiplan4rust::io::RawKind;
    ///
    /// let kind = RawKind::Domain;
    /// assert_eq!(format!("{}", kind), "Domain");
    ///
    /// let kind = RawKind::Problem;
    /// assert_eq!(format!("{}", kind), "Problem");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RawKind::Domain => write!(f, "Domain"),
            RawKind::Problem => write!(f, "Problem"),
        }
    }
}
