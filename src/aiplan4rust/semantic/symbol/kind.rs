//! Defines the different kinds of symbols used within the AI syntax system.
//!
//! This module provides the `Kind` enum, which categorizes symbols based on their roles,
//! such as actions, predicates, variables, and domain-specific constructs.
//! These classifications facilitate semantic analysis, symbol resolution,
//! and other processing steps during syntax and compilation.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Enum representing the various kinds of symbols in the system.
///
/// This enum classifies symbols according to their roles or types within a domain, problem,
/// or plan context. It helps distinguish different symbol types when processing,
/// analyzing, or generating plans.
///
/// # Variants
///
/// - `Action`: Represents an action in the domain (a specific task or operation).
/// - `DASymbol`: Represents a durative action symbol used in temporal domains.
/// - `Method`: Represents a method symbol, defining complex task decompositions.
/// - `Task`: Represents a task symbol, usually part of hierarchical task networks.
/// - `TaskID`: Represents a unique identifier for tasks.
/// - `Constant`: Represents an immutable constant value.
/// - `DomainName`: Represents the domain's name symbol.
/// - `Function`: Represents a function or functor symbol.
/// - `Predicate`: Represents predicate symbols for logical conditions.
/// - `PrimitiveType`: Represents basic data types (e.g., integer, boolean).
/// - `ProblemName`: Represents the name of the syntax problem.
/// - `Requirement`: Represents domain or problem requirements or constraints.
/// - `Variable`: Represents variables that can hold values during execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    /// Represents an action in the domain (e.g., a specific task or operation).
    Action,

    /// Represents a durative action symbol (used in temporal domains).
    DASymbol,

    /// Represents a method used to define decompositions or procedural tasks.
    Method,

    /// Represents a task symbol (typically used in hierarchical task networks).
    Task,

    /// Represents a unique identifier for a task.
    TaskID,

    /// Represents a constant value that does not change.
    Constant,

    /// Represents the name of the domain.
    DomainName,

    /// Represents a function or functor symbol.
    Function,

    /// Represents a predicate symbol (used for logical conditions).
    Predicate,

    /// Represents a primitive data type (e.g., integer, boolean).
    PrimitiveType,

    /// Represents the name of the problem being solved.
    ProblemName,

    /// Represents a requirement or constraint in the domain or problem.
    Requirement,

    /// Represents a variable that can hold values during plan execution.
    Variable,
}

impl fmt::Display for Kind {
    /// Formats the `Kind` enum as a human-readable string.
    ///
    /// Each variant is converted to a descriptive name, suitable for error messages,
    /// logs, or user-facing output.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Action => write!(f, "Action"),
            Kind::DASymbol => write!(f, "Durative Action"),
            Kind::PrimitiveType => write!(f, "Primitive Type"),
            Kind::Predicate => write!(f, "Predicate"),
            Kind::Variable => write!(f, "Variable"),
            Kind::Constant => write!(f, "Constant"),
            Kind::Function => write!(f, "Functor"),
            Kind::DomainName => write!(f, "Domain Name"),
            Kind::ProblemName => write!(f, "Problem Name"),
            Kind::Requirement => write!(f, "Requirement"),
            Kind::Method => write!(f, "Method"),
            Kind::Task => write!(f, "Task"),
            Kind::TaskID => write!(f, "TaskID"),
        }
    }
}
