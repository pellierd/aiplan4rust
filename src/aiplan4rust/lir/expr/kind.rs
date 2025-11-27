//! Module defining the `Kind` enum representing the classification of expression components
//! in the LIR (Logical Intermediate Representation) for AI syntax.
//!
//! # Overview
//!
//! The `Kind` enum categorizes various syntactic and semantic entities used in expressions,
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
//! use crate::aiplan4rust::lir::expr::Kind;
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

use std::fmt;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lir::expr::error::ExprError;
use crate::aiplan4rust::syntax::ast::AstKind;

#[derive(Clone, Debug, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub enum Kind {
    Constant,
    Variable,
    FunctionSymbol,
    PrimitiveType,
    Predicate,
    TaskSymbol,
    PrefName,
    Type,
    TypedList,
    TypedSymbol,
    FunctionTerm,
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
    FComp,
    Assign,
    Operation,
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
    TaskID,
    TaggedTask, // check
    TaskOrderingConstraint, // check
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Kind::Constant => "Constant",
            Kind::Variable => "Variable",
            Kind::FunctionSymbol => "FunctionSymbol",
            Kind::PrimitiveType => "PrimitiveType",
            Kind::Predicate => "Predicate",
            Kind::TaskSymbol => "TaskSymbol",
            Kind::PrefName => "PrefName",
            Kind::Type => "Type",
            Kind::TypedList => "TypedList",
            Kind::TypedSymbol => "TypedSymbol",
            Kind::FunctionTerm => "FunctionTerm",
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
            Kind::FComp => "FComp",
            Kind::Assign => "Assign",
            Kind::Operation => "Operation",
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
            Kind::TaskID => "TaskID",
            Kind::TaggedTask => "TaggedTask",
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
            AstKind::Predicate => Ok(Kind::Predicate),
            AstKind::Variable => Ok(Kind::Variable),
            AstKind::Constant => Ok(Kind::Constant),
            AstKind::FunctionSymbol => Ok(Kind::FunctionSymbol),
            AstKind::TaskSymbol => Ok(Kind::TaskSymbol),
            AstKind::PrefName => Ok(Kind::PrefName),
            AstKind::FunctionTerm => Ok(Kind::FunctionTerm),
            AstKind::Number => Ok(Kind::Number),
            AstKind::AtomicFormula => Ok(Kind::AtomicFormula),
            AstKind::FComp => Ok(Kind::FComp),
            AstKind::Assign => Ok(Kind::Assign),
            AstKind::Operation => Ok(Kind::Operation),
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
            AstKind::TaskID => Ok(Kind::TaskID),
            AstKind::TaggedTask => Ok(Kind::TaggedTask),
            AstKind::Type => Ok(Kind::Type),
            AstKind::PrimitiveType => Ok(Kind::PrimitiveType),
            AstKind::TypedList => Ok(Kind::TypedList),
            AstKind::TypedItem => Ok(Kind::TypedSymbol),
            AstKind::TaskOrderingConstraint => Ok(Kind::TaskOrderingConstraint),
            other => Err(ExprError::unsupported_kind(other)),
        }
    }
}
