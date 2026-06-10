//! # PDDL and HDDL Requirements Enumeration
//!
//! This module defines the `Requirement` enum, representing the various features
//! a PDDL (Planning Domain Definition Language) domain or problem can require.
//!
//! Each variant corresponds to a specific PDDL feature declared via the `:requirements`
//! keyword, affecting the language constructs allowed in the domain/problem definition.
//!
//! The enum supports:
//! - Checking implied atomic requirements (`imply` method).
//! - Converting to static string representations (`as_str`).
//! - Standard formatting with `Display`.
//! - Formatting with an interner via `InternerDisplay` (trivial pass-through here).
//! - Pretty-printing for syntax syntax with indentation (`SyntaxDisplay`).
//!
//! # Example
//!
//! ```rust
//! use your_crate::Requirement;
//!
//! let req = Requirement::Strips;
//! assert_eq!(req.as_str(), "strips");
//! println!("{}", req); // prints "strips"
//! ```

use crate::aiplan4rust::support::interner::{InternerDisplay, SymbolInterner};
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
use crate::aiplan4rust::syntax::lexer::token::{ACTION_COSTS, GOAL_UTILITIES};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use std::collections::HashSet;

use serde::Deserialize;
use serde::Serialize;

use std::fmt;
use std::fmt::Formatter;

/// # PDDL and HDDL Requirements Enum
///
/// The `Requirement` enum represents the various features that a PDDL (Planning Domain Definition
/// Language) or HDDL (Hierarchical Domain Definition Language) domain or problem can declare using
/// the `:requirements` keyword. Each variant corresponds to a specific feature that influences the
/// expressiveness of the syntax formalism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Requirement {
    /// Represents the basic STRIPS formalism, which supports only add and delete effects.
    Strips,
    /// A general requirement encompassing `NegativePreconditions`, `DisjunctivePreconditions`,
    /// `Equality`, `ExistentialPreconditions`, `UniversalPreconditions`, and `ConditionalEffects`.
    Adl,
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
    /// Supports numeric state variables and arithmetic logic.
    NumericFluents,
    /// Supports function with type_checker that differ from number
    ObjectFluents,
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
    /// Supports cost-based syntax, where actions have associated costs.
    ActionCosts,
    /// Represents the marker for Hierarchical Task Network (HTN) definitions in HDDL.
    Hierarchy,
    /// Specifies method preconditions in HTN for HDDL.
    MethodPreconditions,
    /// **[Deprecated]** Non-standard requirement from IPC-2008.
    /// Used for Net Benefit planning; equivalent functionality is now
    /// covered by `:preferences` and `:constraints` in PDDL 3.0+.
    GoalUtilities,
}

impl Requirement {
    /// Returns `true` if the requirement is considered deprecated.
    ///
    /// A requirement is deprecated if it was a non-standard or experimental
    /// extension (e.g., from specific IPC competitions) that has since been
    /// superseded by official PDDL 3.0+ features.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::aiplan4rust::lang::Requirement;
    ///
    /// assert!(Requirement::GoalUtilities.is_deprecated());
    /// assert!(!Requirement::Typing.is_deprecated());
    /// ```
    pub fn is_deprecated(&self) -> bool {
        matches!(self, Requirement::GoalUtilities)
    }

    /// Computes the transitive closure of a set of requirements.
    ///
    /// This function expands the provided set of explicitly declared requirements
    /// into a complete "effective" set by resolving all PDDL hierarchies.
    ///
    /// It acts as the **Source of Truth** for requirement implications: for instance,
    /// if the input contains `:adl`, the resulting closure will include `:typing`,
    /// `:strips`, `:equality`, and all other atomic requirements bundled under the ADL meta-requirement.
    ///
    /// # Arguments
    /// * `declared` - A reference to a `HashSet` of requirements explicitly specified in the source.
    ///
    /// # Returns
    /// A new `HashSet<Requirement>` containing all explicit requirements and their
    /// implicit atomic dependencies.
    pub fn closure(declared: &HashSet<Requirement>) -> HashSet<Requirement> {
        let mut effective = HashSet::new();
        for req in declared {
            // We use extend to merge the vector of implications into the set
            effective.extend(req.imply());
        }
        effective
    }

    /// Checks if a specific atomic requirement is covered by a set of declared requirements,
    /// taking PDDL hierarchy and implications into account.
    ///
    /// This method is optimized for single checks. If you need to check many requirements
    /// against the same set, consider using `closure` once instead.
    ///
    /// # Arguments
    /// * `atomic` - The specific requirement to check for coverage.
    /// * `declared` - The set of requirements explicitly declared in the PDDL source.
    ///
    /// # Returns
    /// `true` if the requirement is explicitly declared or implicitly covered by a
    /// meta-requirement (like :adl).
    pub fn is_covered_by(atomic: &Requirement, declared: &HashSet<Requirement>) -> bool {
        // Fast path: direct match
        if declared.contains(atomic) {
            return true;
        }

        // Logic path: check if any declared requirement implies the atomic one
        // We iterate through declared requirements and check their implications
        declared.iter().any(|req| {
            match (req, atomic) {
                // ADL covers almost everything in the base spec
                (Requirement::Adl, _) => match atomic {
                    Requirement::Strips
                    | Requirement::Typing
                    | Requirement::NegativePreconditions
                    | Requirement::DisjunctivePreconditions
                    | Requirement::Equality
                    | Requirement::QuantifiedPreconditions
                    | Requirement::ExistentialPreconditions
                    | Requirement::UniversalPreconditions
                    | Requirement::ConditionalEffects => true,
                    _ => false,
                },

                // Quantified Preconditions covers its children
                (Requirement::QuantifiedPreconditions, _) => match atomic {
                    Requirement::ExistentialPreconditions | Requirement::UniversalPreconditions => {
                        true
                    }
                    _ => false,
                },

                // Fluents covers numeric and object variants
                (Requirement::Fluents, _) => match atomic {
                    Requirement::NumericFluents | Requirement::ObjectFluents => true,
                    _ => false,
                },

                // Timed Initial Literals usually implies/requires Durative Actions
                (Requirement::TimedInitialLiterals, Requirement::DurativeActions) => true,

                // Default: no match found for this specific pair
                _ => false,
            }
        })
    }

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
            Requirement::GoalUtilities => GOAL_UTILITIES,
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
impl InternerDisplay for Requirement {
    /// Formats the `Requirement` using the given formatter.
    ///
    /// Delegates directly to the standard `Display` implementation,
    /// ignoring the `interner` parameter.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter.
    /// * `_interner` - The interner, unused in this implementation.
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, _interner: &SymbolInterner) -> fmt::Result {
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
impl SyntaxInternerDisplay for Requirement {
    /// Formats the `Requirement` for syntax syntax display.
    ///
    /// Delegates the formatting to the `Display` trait implementation.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output to.
    /// * `_interner` - The string interner (unused in this implementation).
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        _interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        fmt::Display::fmt(self, f)
    }
}
