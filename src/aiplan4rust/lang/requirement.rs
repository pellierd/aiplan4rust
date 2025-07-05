use crate::aiplan4rust::syntax::lexer::token::ACTION_COSTS;
use crate::aiplan4rust::syntax::lexer::token::ADL;
use crate::aiplan4rust::syntax::lexer::token::CONDITIONAL_EFFECTS;
use crate::aiplan4rust::syntax::lexer::token::CONSTRAINTS;
use crate::aiplan4rust::syntax::lexer::token::DERIVED_PREDICATES;
use crate::aiplan4rust::syntax::lexer::token::DISJUNCTIVE_PRECONDITION;
use crate::aiplan4rust::syntax::lexer::token::DURATIVE_ACTIONS;
use crate::aiplan4rust::syntax::lexer::token::DURATIVE_INEQUALITIES;
use crate::aiplan4rust::syntax::lexer::token::EQUALITY;
use crate::aiplan4rust::syntax::lexer::token::EXISTENTIAL_PRECONDITIONS;
use crate::aiplan4rust::syntax::lexer::token::FLUENTS;
use crate::aiplan4rust::syntax::lexer::token::HIERARCHY;
use crate::aiplan4rust::syntax::lexer::token::METHOD_PRECONDITIONS;
use crate::aiplan4rust::syntax::lexer::token::NEGATIVE_PRECONDITION;
use crate::aiplan4rust::syntax::lexer::token::NUMERIC_FLUENTS;
use crate::aiplan4rust::syntax::lexer::token::OBJECT_FLUENTS;
use crate::aiplan4rust::syntax::lexer::token::PREFERENCES;
use crate::aiplan4rust::syntax::lexer::token::QUANTIFIED_PRECONDITIONS;
use crate::aiplan4rust::syntax::lexer::token::STRIPS;
use crate::aiplan4rust::syntax::lexer::token::TIME_INITIAL_LITERALS;
use crate::aiplan4rust::syntax::lexer::token::TYPING;
use crate::aiplan4rust::syntax::lexer::token::UNIVERSAL_PRECONDITIONS;
use crate::aiplan4rust::syntax::PlanningSyntaxDisplay;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};

use serde::Deserialize;
use serde::Serialize;

use std::fmt;
use std::fmt::Formatter;

/// # PDDL Requirements Enum
///
/// The `Requirement` enum represents the various features that a PDDL (Planning Domain Definition
/// Language) domain or problem can declare using the `:requirements` keyword. Each variant
/// corresponds to a specific feature that influences the expressiveness of the planning formalism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Supports numeric state variables and arithmetic expr.
    NumericFluents,
    /// Supports function with type that differ from number
    ObjectFluents,
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

impl Requirement {
    // Returns all atomic requirements implied by this requirement, including itself.
    pub fn imply(&self) -> Vec<Requirement> {
        match self {
            Requirement::QuantifiedPreconditions => vec![
                Requirement::QuantifiedPreconditions,
                Requirement::ExistentialPreconditions,
                Requirement::UniversalPreconditions,
            ],
            Requirement::Fluents => vec![
                Requirement::Fluents,
                Requirement::NumericFluents,
                Requirement::ObjectFluents,
            ],
            Requirement::Adl => vec![
                Requirement::Adl,
                Requirement::Strips,
                Requirement::Typing,
                Requirement::NegativePreconditions,
                Requirement::DisjunctivePreconditions,
                Requirement::Equality,
                Requirement::QuantifiedPreconditions,
                Requirement::ExistentialPreconditions,
                Requirement::UniversalPreconditions,
                Requirement::ConditionalEffects,
            ],
            Requirement::TimedInitialLiterals => vec![
                Requirement::TimedInitialLiterals,
                Requirement::DurativeActions,
            ],
            _ => vec![self.clone()],
        }
    }

    /// Returns the string representation of the requirement.
    ///
    /// Unlike `Display`, this method returns a static string slice (`&'static str`),
    /// which is useful when the string is needed without allocation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Requirement::Strips => STRIPS,
            Requirement::Typing => TYPING,
            Requirement::NegativePreconditions => NEGATIVE_PRECONDITION,
            Requirement::DisjunctivePreconditions => DISJUNCTIVE_PRECONDITION,
            Requirement::Equality => EQUALITY,
            Requirement::ExistentialPreconditions => EXISTENTIAL_PRECONDITIONS,
            Requirement::UniversalPreconditions => UNIVERSAL_PRECONDITIONS,
            Requirement::QuantifiedPreconditions => QUANTIFIED_PRECONDITIONS,
            Requirement::ConditionalEffects => CONDITIONAL_EFFECTS,
            Requirement::Fluents => FLUENTS,
            Requirement::NumericFluents => NUMERIC_FLUENTS,
            Requirement::ObjectFluents => OBJECT_FLUENTS,
            Requirement::Adl => ADL,
            Requirement::DurativeActions => DURATIVE_ACTIONS,
            Requirement::DurationInequalities => DURATIVE_INEQUALITIES,
            Requirement::ContinuousEffects => CONDITIONAL_EFFECTS,
            Requirement::DerivedPredicates => DERIVED_PREDICATES,
            Requirement::TimedInitialLiterals => TIME_INITIAL_LITERALS,
            Requirement::Preferences => PREFERENCES,
            Requirement::Constraints => CONSTRAINTS,
            Requirement::ActionCosts => ACTION_COSTS,
            Requirement::Hierarchy => HIERARCHY,
            Requirement::MethodPreconditions => METHOD_PRECONDITIONS,
        }
    }
}

/// Implements the `Display` trait for `Requirement`.
///
/// This implementation formats a `Requirement` value by
/// converting it to its corresponding lexeme string,
/// as defined by the PDDL specification.
///
/// # Example
///
/// ```
/// let req = Requirement::Mandatory;
/// assert_eq!(req.to_string(), "Mandatory");
/// ```
impl fmt::Display for Requirement {
    /// Formats the `Requirement` as its corresponding lexeme string.
    ///
    /// Writes the string representation of the `Requirement` variant
    /// to the given formatter.
    ///
    /// # Parameters
    ///
    /// * `f` - The formatter used to output the formatted string.
    ///
    /// # Returns
    ///
    /// A `fmt::Result` indicating success or failure.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}


/// Implements the `DisplayWithInterner` trait for `Requirement`.
///
/// This implementation formats a `Requirement` value by
/// delegating to its standard `Display` implementation,
/// since no interner-based resolution is needed.
///
/// The `interner` parameter is not used.
///
/// # Example
///
/// ```
/// let req = Requirement::Mandatory;
/// let s = req.to_string_with_interner(&interner);
/// assert_eq!(s, "Mandatory");
/// ```
impl DisplayWithInterner for Requirement {
    /// Formats the `Requirement` using the given formatter.
    ///
    /// Delegates directly to the standard `Display` implementation,
    /// ignoring the `interner` parameter.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    /// * `_interner` - The interner, unused in this implementation.
    fn fmt_with(&self, f: &mut Formatter<'_>, _interner: &StringInterner) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}


/// Implements the `PlanningSyntaxDisplay` trait for `Requirement`.
///
/// This implementation provides user-facing syntax formatting for `Requirement` values.
///
/// The `fmt_planning` method simply delegates to the standard `Display` implementation,
/// ensuring consistent formatting across different display traits.
///
/// # Example
///
/// ```
/// let req = Requirement::Mandatory;
/// let s = req.to_string_syntax(&interner);
/// assert_eq!(s, "Mandatory");
/// ```
impl PlanningSyntaxDisplay for Requirement {
    /// Formats the `Requirement` for planning syntax display.
    ///
    /// Delegates the formatting to the `Display` trait implementation.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    /// * `_interner` - The string interner (unused in this implementation).
    fn fmt_planning(&self, f: &mut Formatter<'_>, _interner: &StringInterner) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
