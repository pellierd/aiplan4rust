use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Enum representing the different kinds of symbols in the system.
///
/// This enum categorizes the symbols based on their role or type within a domain, problem, or plan.
/// It is used to distinguish between different symbol types when processing or analyzing a symbol.
///
/// # Variants
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    /// Represents an action in the domain (e.g., a specific task or operation).
    Action,

    /// Represents a symbol used in the domain description (e.g., a symbol specific to a domain
    /// model).
    DASymbol,

    /// Represents a symbol associated with a method in the domain, used for defining methods
    /// that perform actions or tasks, often related to processes or operations in the domain.
    Method,

    /// Represents a symbol associated with a task in the domain, typically used to define a
    /// specific task or operation that can be planned and executed within the system.
    Task,

    /// Represents a unique identifier for a task in the domain, used for referencing tasks
    /// within the domain model.
    TaskID,

    /// Represents a constant value that does not change.
    Constant,

    /// Represents the name of a domain.
    DomainName,

    /// Represents a function symbol.
    Function,

    /// Represents a predicate symbol (typically used for logical conditions).
    Predicate,

    /// Represents a basic data type (e.g., integer, boolean).
    PrimitiveType,

    /// Represents the name of the problem being solved (e.g., a problem definition).
    ProblemName,

    /// Represents a requirement or constraint within a domain or problem context.
    Requirement,

    /// Represents a variable that can hold different values during execution.
    Variable,
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolKind::Action => write!(f, "Action"),
            SymbolKind::DASymbol => write!(f, "Durative Action"),
            SymbolKind::PrimitiveType => write!(f, "Primitive Type"),
            SymbolKind::Predicate => write!(f, "Predicate"),
            SymbolKind::Variable => write!(f, "Variable"),
            SymbolKind::Constant => write!(f, "Constant"),
            SymbolKind::Function => write!(f, "Functor"),
            SymbolKind::DomainName => write!(f, "Domain Name"),
            SymbolKind::ProblemName => write!(f, "Problem Name"),
            SymbolKind::Requirement => write!(f, "Requirement"),
            // Add for HDDL
            SymbolKind::Method => write!(f, "Method"),
            SymbolKind::Task => write!(f, "Task"),
            SymbolKind::TaskID => write!(f, "TaskID"),
        }
    }
}
