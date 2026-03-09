//! Inertia Classification for Lifted Planning Symbols (Koehler's Rules).
//!
//! This module tracks the stability of predicates and functions based on
//! their presence in action effects. By using inertia flags, we can identify
//! symbols that are both Positive and Negative Inert (technically static),
//! significantly optimizing the grounding and reachability analysis.

use std::fmt;

/// Represents the stability properties of a predicate or function.
///
/// According to IPP (Koehler), inertia is defined by the absence of specific types of effects.
/// - **Positive Inertia**: The symbol is never added (remains False if initially False).
/// - **Negative Inertia**: The symbol is never deleted (remains True if initially True).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Inertia {
    /// If true, the symbol NEVER appears in an 'ADD' (or 'assign'/'increase') effect.
    /// It remains in its initial state or remains False/Zero if not in Init.
    pub is_positive_inert: bool,

    /// If true, the symbol NEVER appears in a 'DEL' (or 'assign'/'decrease') effect.
    /// It remains in its initial state or remains True if it was already True.
    pub is_negative_inert: bool,
}

impl Inertia {
    // --- Constructors (Factories) ---

    /// Creates a state of both **Positive AND Negative Inertia**.
    ///
    /// # Returns
    /// An `Inertia` instance where no actions can modify the symbol.
    pub fn positive_negative() -> Self {
        Self { is_positive_inert: true, is_negative_inert: true }
    }

    /// Creates a **Fluent** state (neither positive nor negative inert).
    ///
    /// # Returns
    /// An `Inertia` instance where the symbol can be both added and removed.
    pub fn fluent() -> Self {
        Self { is_positive_inert: false, is_negative_inert: false }
    }

    /// Creates a **Positive Inert** state.
    ///
    /// # Returns
    /// An `Inertia` instance where the symbol can be removed, but NEVER added.
    pub fn positive() -> Self {
        Self { is_positive_inert: true, is_negative_inert: false }
    }

    /// Creates a **Negative Inert** state.
    ///
    /// # Returns
    /// An `Inertia` instance where the symbol can be added, but NEVER removed.
    pub fn negative() -> Self {
        Self { is_positive_inert: false, is_negative_inert: true }
    }

    // --- Queries (Predicates) ---

    /// Returns true if the symbol is both **Positive and Negative Inert**.
    ///
    /// # Returns
    /// `true` if the symbol's value is guaranteed to never change from the Initial State.
    pub fn is_positive_negative(&self) -> bool {
        self.is_positive_inert && self.is_negative_inert
    }

    /// Returns true if the symbol is **Fluent**.
    ///
    /// # Returns
    /// `true` if the symbol can undergo both additions and deletions.
    pub fn is_fluent(&self) -> bool {
        !self.is_positive_inert && !self.is_negative_inert
    }

    /// Returns true if the symbol is **Positive Inert**.
    ///
    /// # Returns
    /// `true` if the symbol can never be added or increased by any action.
    pub fn is_positive(&self) -> bool {
        self.is_positive_inert
    }

    /// Returns true if the symbol is **Negative Inert**.
    ///
    /// # Returns
    /// `true` if the symbol can never be removed or decreased by any action.
    pub fn is_negative(&self) -> bool {
        self.is_negative_inert
    }
}

impl Default for Inertia {
    /// Defaults to **positive_negative**, assuming no changes until an effect is detected.
    fn default() -> Self {
        Self::positive_negative()
    }
}

impl fmt::Display for Inertia {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match (self.is_positive_inert, self.is_negative_inert) {
            (true, true)   => "POSITIVE_NEGATIVE_INERTIA",
            (true, false)  => "POSITIVE_INERTIA",
            (false, true)  => "NEGATIVE_INERTIA",
            (false, false) => "FLUENT",
        };
        write!(f, "{}", label)
    }
}
