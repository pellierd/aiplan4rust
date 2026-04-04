//! Defines the different kinds of symbols used within the AI syntax system.
//!
//! This module provides the `Kind` enum, which categorizes symbols based on their roles,
//! such as actions, predicates, variables, and domain-specific constructs.
//! These classifications facilitate semantic analysis, symbol resolution,
//! and other processing steps during syntax and compilation.

use crate::aiplan4rust::semantic::SemanticError;
use crate::aiplan4rust::syntax::ast::AstKind;
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

    /// Represents a derived predicate symbol.
    //DerivedPredicate,

    /// Represents a primitive data typing (e.g., integer, boolean).
    PrimitiveType,

    /// Represents the name of the problem being solved.
    ProblemName,

    /// Represents a requirement or constraint in the domain or problem.
    Requirement,

    /// Represents a variable that can hold values during plan execution.
    Variable,
}

impl Kind {
    /*//// Returns `true` if two symbols are allowed to share the same name (identifier)
    /// in a case-insensitive context.
    ///
    /// This logic is essential for PDDL/HDDL compatibility, allowing common
    /// naming conventions while preventing critical unification ambiguities.
    ///
    /// # Sharing Rules
    ///
    /// * **Same Kind (`self == other`):**
    ///     * Returns `true` for **Constants** (allows a Problem to redeclare a Domain constant).
    ///     * Returns `true` for **Derived Predicates**. This is crucial as PDDL allows
    ///       multiple `:derived` axioms to define the same predicate (logical OR).
    ///     * Returns `false` for all other kinds (e.g., two Actions cannot share a name).
    ///
    /// * **Predicate vs Derived Predicate:**
    ///     * Returns `true`. A predicate can be declared in the `(:predicates)` block
    ///       and then further defined or refined in one or more `(:derived)` blocks.
    ///
    /// * **Task vs Action (HDDL):**
    ///     * Returns `true`. In hierarchical planning, it is a common pattern for
    ///       an abstract Task to share its name with a concrete Action that implements it.
    ///
    /// * **Type vs Constant/Object:**
    ///     * Returns `true`. This supports the "Singleton" idiom where a unique
    ///       object shares the name of its type (e.g., `fireExtinguisher - fireExtinguisher`).
    ///
    /// * **Type vs Predicate:**
    ///     * Returns `true`. While syntactically distinct due to parentheses `(p ...)`,
    ///       sharing is allowed but may trigger a compiler warning elsewhere.
    ///
    /// * **Structural Names (Domain/Problem):**
    ///     * Returns `true` when compared against any logical symbol. This allows
    ///       a predicate or object to safely share the name of the Domain or Problem itself.
    ///
    /// * **Variables (`?x`):**
    ///     * Always returns `false`. Variables never share their namespace to ensure
    ///       safe unification and avoid collisions with static symbols.
    ///
    /// * **Default:**
    ///     * Returns `false` for any other combination (e.g., Task vs Constant)
    ///       to maintain model clarity.
    pub fn can_share_name_space_with(&self, other: &Self) -> bool {
        // 1. Handle identical kinds
        if self == other {
            // Authorized for constants (Domain/Problem redeclaration)
            // AND for DerivedPredicates (multiple axioms for the same predicate)
            return matches!(self, Kind::Constant | Kind::DerivedPredicate);
        }

        // 2. Handle authorized mixed pairs
        match (self, other) {
            // Structural identifiers (Domain/Problem names) should not collide with logical symbols.
            (Kind::DomainName, _) | (_, Kind::DomainName) => true,
            (Kind::ProblemName, _) | (_, Kind::ProblemName) => true,

            // Singleton convention: Type and Constant/Object
            (Kind::PrimitiveType, Kind::Constant) | (Kind::Constant, Kind::PrimitiveType) => true,

            // Manageable syntactic ambiguity: Type and Predicate
            (Kind::PrimitiveType, Kind::Predicate) | (Kind::Predicate, Kind::PrimitiveType) => true,

            // PDDL Derived Predicates: A predicate can be declared in (:predicates)
            // and defined in (:derived)
            (Kind::Predicate, Kind::DerivedPredicate)
            | (Kind::DerivedPredicate, Kind::Predicate) => true,

            // HDDL: A Task and an Action often share the same name.
            (Kind::Task, Kind::Action) | (Kind::Action, Kind::Task) => true,

            // Strict safety: Variables never share their namespace
            (Kind::Variable, _) | (_, Kind::Variable) => false,

            // Default: Strict (Task, Method, etc.)
            _ => false,
        }
    }*/
}

impl TryFrom<AstKind> for Kind {
    type Error = SemanticError;

    fn try_from(ast_kind: AstKind) -> Result<Self, Self::Error> {
        match ast_kind {
            // --- SYMBOLS & VARIABLES ---
            AstKind::Variable => Ok(Kind::Variable),
            AstKind::Object => Ok(Kind::Constant),
            AstKind::FunctionSymbol => Ok(Kind::Function),
            AstKind::PredicateSymbol => Ok(Kind::Predicate),

            // --- HIERARCHICAL & PROCEDURAL ---
            AstKind::ActionSymbol => Ok(Kind::Action),
            AstKind::MethodSymbol => Ok(Kind::Method),
            AstKind::Task => Ok(Kind::Task),
            AstKind::TaskLabel => Ok(Kind::TaskID),

            // --- TEMPORAL & TYPES ---
            AstKind::PrimitiveType => Ok(Kind::PrimitiveType),

            // --- TOP-LEVEL STRUCTURES ---
            AstKind::DomainName => Ok(Kind::DomainName),
            AstKind::ProblemName => Ok(Kind::ProblemName),
            AstKind::Requirement => Ok(Kind::Requirement),

            // On utilise ta fonction helper pour transformer le AstKind inconnu
            // en une SemanticError tracée.
            found => Err(SemanticError::ast_kind_conversion(found)),
        }
    }
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
