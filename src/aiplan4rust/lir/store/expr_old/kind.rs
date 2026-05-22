//! Module defining the `Kind` enum representing the classification of expression components
//! in the LIR (Logical Intermediate Representation) for AI syntax.
//!
//! # Overview
//!
//! The `Kind` enum categorizes various syntactic and semantic entities used in logic,
//! including logical operators, terms, predicates, task symbols, and temporal constructs.
//! It serves as an abstraction layer over raw AST kinds (`AstKind`) used during parsing,
//! enabling a more domain-specific representation of expression nodes.
//!
//! This module also provides:
//! - Conversion (`TryFrom`) from the generic `AstKind` into the more specialized `Kind`,
//!   with error handling for unsupported kinds.
//! - A `Display` implementation for readable string representation of each kind.
//!
//! # Enum Variants
//!
//! Variants include (but are not limited to):
//! - Logical operators: `And`, `Or`, `Not`, `Imply`, `Forall`, `Exists`
//! - Constants and variables: `Constant`, `Variable`
//! - Function and predicate symbols: `FunctionSymbol`, `Predicate`
//! - Task-related constructs: `TaskSymbol`, `Task`, `TaskID`, `TaggedTask`, `TaskOrderingConstraint`
//! - Temporal and metric constructs: `AtStart`, `AtEnd`, `Always`, `Sometime`, `Metric`, `TotalTime`
//! - Types and typing constructs: `Type`, `PrimitiveType`, `TypedList`, `TypedSymbol`
//!
//! # Conversion from AST
//!
//! The `TryFrom<AstKind>` implementation attempts to convert a generic AST kind into
//! a `Kind`. Unsupported AST kinds result in an `ExprError` to signal that conversion
//! is not possible in the current context.
//!
//! # Usage Example
//!
//! ```rust
//! use crate::aiplan4rust::lir::logic::Kind;
//! use crate::aiplan4rust::syntax::ast::AstKind;
//! use std::convert::TryFrom;
//!
//! let ast_kind = AstKind::And;
//! let kind = Kind::try_from(ast_kind).expect("Supported kind");
//! assert_eq!(kind.to_string(), "And");
//! ```
//!
//! # Errors
//!
//! Converting from `AstKind` to `Kind` may fail with [`ExprError`] if the AST kind
//! is unsupported.
//!

use crate::aiplan4rust::lir::store::expr_old::error::ExprError;
use crate::aiplan4rust::syntax::ast::AstKind;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum Kind {
    Object,
    Variable,
    FunctionSymbol,
    PredicateSymbol,
    TaskSymbol,
    PrefName,
    Function,
    Number,
    AtomicFormula,
    And,
    #[default]
    Or,
    Not,
    Imply,
    Forall,
    Exists,
    Preference,
    When,
    Comparison,
    Assignment,
    Arithmetic,
    AtStart,
    AtEnd,
    Overall,
    Always,
    Sometime,
    Within,
    AtMostOnce,
    SometimeAfter,
    SometimeBefore,
    AlwaysWithin,
    HoldDuring,
    HoldAfter,
    TimedInitialLiteral,
    Metric,
    TotalTime,
    IsViolated,
    Length,
    Serial,
    Parallel,
    Task,
    TaskLabel,
    LabeledTask,            // check
    TaskOrderingConstraint, // check
}

impl Kind {
    pub fn to_pddl_keyword(&self) -> &'static str {
        match self {
            // Leaves and terminals (content is handled by the Content module)
            Kind::Object
            | Kind::Variable
            | Kind::FunctionSymbol
            | Kind::PredicateSymbol
            | Kind::TaskSymbol
            | Kind::PrefName
            | Kind::Function
            | Kind::Number
            | Kind::AtomicFormula
            | Kind::Task
            | Kind::TaskLabel
            | Kind::LabeledTask => "",

            // Logical Connectives
            Kind::And => "and",
            Kind::Or => "or",
            Kind::Not => "not",
            Kind::Imply => "imply",
            Kind::Forall => "forall",
            Kind::Exists => "exists",
            Kind::When => "when",

            // Quantifiers and Preferences
            Kind::Preference => "preference",
            Kind::IsViolated => "is-violated",

            // Numerical Comparisons and Operations
            // Note: Usually handled by Content (e.g., <, >, +, -)
            Kind::Comparison | Kind::Arithmetic => "",
            Kind::Assignment => "",

            // Temporal (PDDL 2.1+)
            Kind::AtStart => "at start",
            Kind::AtEnd => "at end",
            Kind::Overall => "overall",

            // Modal Constraints / Trajectories (PDDL 3.0)
            Kind::Always => "always",
            Kind::Sometime => "sometime",
            Kind::Within => "within",
            Kind::AtMostOnce => "at-most-once",
            Kind::SometimeAfter => "sometime-after",
            Kind::SometimeBefore => "sometime-before",
            Kind::AlwaysWithin => "always-within",
            Kind::HoldDuring => "hold-during",
            Kind::HoldAfter => "hold-after",

            // Temporal Planning and Metrics
            Kind::TimedInitialLiteral => "at",
            Kind::Metric => "metric",
            Kind::TotalTime => "total-time",

            // HTN and specific extensions
            Kind::TaskOrderingConstraint => "ordering",
            Kind::Serial => "serial",
            Kind::Parallel => "parallel",
            Kind::Length => "length",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Kind::Object => "Constant",
            Kind::Variable => "Variable",
            Kind::FunctionSymbol => "FunctionSymbol",
            Kind::PredicateSymbol => "Predicate",
            Kind::TaskSymbol => "TaskSymbol",
            Kind::PrefName => "PrefName",
            Kind::Function => "FunctionTerm",
            Kind::Number => "Number",
            Kind::AtomicFormula => "AtomicFormula",
            Kind::And => "And",
            Kind::Or => "Or",
            Kind::Not => "Not",
            Kind::Imply => "Imply",
            Kind::Forall => "Forall",
            Kind::Exists => "Exists",
            Kind::Preference => "Preference",
            Kind::When => "When",
            Kind::Comparison => "FComp",
            Kind::Assignment => "Assign",
            Kind::Arithmetic => "Operation",
            Kind::AtStart => "AtStart",
            Kind::AtEnd => "AtEnd",
            Kind::Overall => "Overall",
            Kind::Always => "Always",
            Kind::Sometime => "Sometime",
            Kind::Within => "Within",
            Kind::AtMostOnce => "AtMostOnce",
            Kind::SometimeAfter => "SometimeAfter",
            Kind::SometimeBefore => "SometimeBefore",
            Kind::AlwaysWithin => "AlwaysWithin",
            Kind::HoldDuring => "HoldDuring",
            Kind::HoldAfter => "HoldAfter",
            Kind::TimedInitialLiteral => "TimedInitialLiteral",
            Kind::Metric => "Metric",
            Kind::TotalTime => "TotalTime",
            Kind::IsViolated => "IsViolated",
            Kind::Length => "Length",
            Kind::Serial => "Serial",
            Kind::Parallel => "Parallel",
            Kind::Task => "Task",
            Kind::TaskLabel => "TaskID",
            Kind::LabeledTask => "TaggedTask",
            Kind::TaskOrderingConstraint => "TaskOrderingConstraint",
        };
        write!(f, "{}", s)
    }
}

impl TryFrom<AstKind> for Kind {
    type Error = ExprError;

    fn try_from(kind: AstKind) -> Result<Self, Self::Error> {
        match kind {
            AstKind::And => Ok(Kind::And),
            AstKind::Or => Ok(Kind::Or),
            AstKind::Not => Ok(Kind::Not),
            AstKind::Imply => Ok(Kind::Imply),
            AstKind::Forall => Ok(Kind::Forall),
            AstKind::Exists => Ok(Kind::Exists),
            AstKind::PredicateSymbol => Ok(Kind::PredicateSymbol),
            AstKind::Variable => Ok(Kind::Variable),
            AstKind::Object => Ok(Kind::Object),
            AstKind::FunctionSymbol => Ok(Kind::FunctionSymbol),
            AstKind::TaskSymbol => Ok(Kind::TaskSymbol),
            AstKind::PrefName => Ok(Kind::PrefName),
            AstKind::Function => Ok(Kind::Function),
            AstKind::Number => Ok(Kind::Number),
            AstKind::AtomicFormula => Ok(Kind::AtomicFormula),
            AstKind::Comparison => Ok(Kind::Comparison),
            AstKind::Assignment => Ok(Kind::Assignment),
            AstKind::Arithmetic => Ok(Kind::Arithmetic),
            AstKind::AtStart => Ok(Kind::AtStart),
            AstKind::AtEnd => Ok(Kind::AtEnd),
            AstKind::Overall => Ok(Kind::Overall),
            AstKind::Always => Ok(Kind::Always),
            AstKind::Sometime => Ok(Kind::Sometime),
            AstKind::Within => Ok(Kind::Within),
            AstKind::AtMostOnce => Ok(Kind::AtMostOnce),
            AstKind::SometimeAfter => Ok(Kind::SometimeAfter),
            AstKind::SometimeBefore => Ok(Kind::SometimeBefore),
            AstKind::AlwaysWithin => Ok(Kind::AlwaysWithin),
            AstKind::HoldDuring => Ok(Kind::HoldDuring),
            AstKind::HoldAfter => Ok(Kind::HoldAfter),
            AstKind::TimedInitialLiteral => Ok(Kind::TimedInitialLiteral),
            AstKind::Metric => Ok(Kind::Metric),
            AstKind::TotalTime => Ok(Kind::TotalTime),
            AstKind::IsViolated => Ok(Kind::IsViolated),
            AstKind::Length => Ok(Kind::Length),
            AstKind::Serial => Ok(Kind::Serial),
            AstKind::Parallel => Ok(Kind::Parallel),
            AstKind::Task => Ok(Kind::Task),
            AstKind::TaskLabel => Ok(Kind::TaskLabel),
            AstKind::LabeledTask => Ok(Kind::LabeledTask),
            AstKind::TaskOrderingConstraint => Ok(Kind::TaskOrderingConstraint),
            other => Err(ExprError::invalid_ast_node(other)),
        }
    }
}
