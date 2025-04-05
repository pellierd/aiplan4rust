use crate::aiplan4rust::pddl_display::PDDLDisplay;
use crate::aiplan4rust::syntax::lexer::token::{
    ACTION_COSTS, ADL, CONDITIONAL_EFFECTS, CONSTRAINTS, DERIVED_PREDICATES,
    DISJUNCTIVE_PRECONDITION, DURATIVE_ACTIONS, DURATIVE_INEQUALITIES, EQUALITY,
    EXISTENTIAL_PRECONDITIONS, FLUENTS, HIERARCHY, METHOD_PRECONDITIONS, NEGATIVE_PRECONDITION,
    NUMERIC_FLUENTS, PREFERENCES, QUANTIFIED_PRECONDITIONS, STRIPS, TIME_INITIAL_LITERALS, TYPING,
    UNIVERSAL_PRECONDITIONS,
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// # PDDL Requirements Enum
///
/// The `Requirement` enum represents the various features that a PDDL (Planning Domain Definition
/// Language) domain or problem can declare using the `:requirements` keyword. Each variant
/// corresponds to a specific feature that influences the expressiveness of the planning formalism.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Requirement {
    /// Represents the basic STRIPS formalism, which supports only add and delete effects.
    Strips,
    /// Enables the use of typed objects and variables.
    Typing,
    /// Allows the use of negated atoms in preconditions.
    NegativePreconditions,
    /// Supports logical OR (`or`) in preconditions.
    DisjunctivePreconditions,
    /// Enables the use of equality (`=`) in conditions.
    Equality,
    /// Supports existential quantification (`exists`) in preconditions.
    ExistentialPreconditions,
    /// Supports universal quantification (`forall`) in preconditions.
    UniversalPreconditions,
    /// A general requirement covering both existential and universal quantification.
    QuantifiedPreconditions,
    /// Allows conditional (`when`) effects in action definitions.
    ConditionalEffects,
    /// Enables numeric state variables (deprecated in favor of `NumericFluents`).
    Fluents,
    /// Supports numeric state variables and arithmetic expressions.
    NumericFluents,
    /// A general requirement encompassing `NegativePreconditions`, `DisjunctivePreconditions`,
    /// `Equality`, `ExistentialPreconditions`, `UniversalPreconditions`, and `ConditionalEffects`.
    Adl,
    /// Allows the definition of durative (temporal) actions.
    DurativeActions,
    /// Enables constraints on action durations.
    DurationInequalities,
    /// Supports continuous change effects over time.
    ContinuousEffects,
    /// Allows the definition of derived predicates (`:derived`).
    DerivedPredicates,
    /// Supports `at` literals for specifying facts that become true at a specific time.
    TimedInitialLiterals,
    /// Introduces soft constraints using `:preferences`.
    Preferences,
    /// Enables global constraints on plans.
    Constraints,
    /// Supports cost-based planning, where actions have associated costs.
    ActionCosts,
    /// Represents the marker for Hierarchical Task Network (HTN) definitions in HDDL.
    Hierarchy,
    /// Specifies method preconditions in HTN for HDDL.
    MethodPreconditions,
}

impl fmt::Display for Requirement {
    /// Formats the `Requirement` as its corresponding lexeme string.
    ///
    /// This implementation converts each variant of the `Requirement` enum into
    /// its predefined string representation, typically used in PDDL files.
    ///
    /// # Parameters
    /// - `f`: The formatter used to output the formatted string.
    ///
    /// # Returns
    /// - `fmt::Result`: The result of writing the formatted output.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Requirement::Strips => write!(f, "{}", STRIPS),
            Requirement::Typing => write!(f, "{}", TYPING),
            Requirement::NegativePreconditions => write!(f, "{}", NEGATIVE_PRECONDITION),
            Requirement::DisjunctivePreconditions => write!(f, "{}", DISJUNCTIVE_PRECONDITION),
            Requirement::Equality => write!(f, "{}", EQUALITY),
            Requirement::ExistentialPreconditions => write!(f, "{}", EXISTENTIAL_PRECONDITIONS),
            Requirement::UniversalPreconditions => write!(f, "{}", UNIVERSAL_PRECONDITIONS),
            Requirement::QuantifiedPreconditions => write!(f, "{}", QUANTIFIED_PRECONDITIONS),
            Requirement::ConditionalEffects => write!(f, "{}", CONDITIONAL_EFFECTS),
            Requirement::Fluents => write!(f, "{}", FLUENTS),
            Requirement::NumericFluents => write!(f, "{}", NUMERIC_FLUENTS),
            Requirement::Adl => write!(f, "{}", ADL),
            Requirement::DurativeActions => write!(f, "{}", DURATIVE_ACTIONS),
            Requirement::DurationInequalities => write!(f, "{}", DURATIVE_INEQUALITIES),
            Requirement::ContinuousEffects => write!(f, "{}", CONDITIONAL_EFFECTS),
            Requirement::DerivedPredicates => write!(f, "{}", DERIVED_PREDICATES),
            Requirement::TimedInitialLiterals => write!(f, "{}", TIME_INITIAL_LITERALS),
            Requirement::Preferences => write!(f, "{}", PREFERENCES),
            Requirement::Constraints => write!(f, "{}", CONSTRAINTS),
            Requirement::ActionCosts => write!(f, "{}", ACTION_COSTS),
            Requirement::Hierarchy => write!(f, "{}", HIERARCHY),
            Requirement::MethodPreconditions => write!(f, "{}", METHOD_PRECONDITIONS),
        }
    }
}

impl PDDLDisplay for Requirement {
    /// Converts the `Requirement` into its PDDL-compliant string representation.
    ///
    /// This function leverages the `Display` trait implementation to generate
    /// the corresponding PDDL lexeme for a given `Requirement` variant.
    ///
    /// # Returns
    /// - A `String`
    fn to_pddl_string(&self) -> String {
        format!("{}", self)
    }
}
