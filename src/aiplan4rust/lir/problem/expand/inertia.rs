use std::fmt;

/// Represents the static truth value of a LIR expression according to Koehler's Pruning rules.
///
/// Inertia analysis identifies predicates that are "static" (never change their value
/// during plan execution) versus "fluent" (dynamic). This classification is a
/// cornerstone of efficient grounding in PDDL/HDDL solvers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Inertia {
    /// Static fact present in the initial state (Always True).
    ///
    /// If a component of a disjunction (OR) is `Positive`, the entire expression
    /// simplifies to `True`. In an existential quantifier ($\exists$), encountering
    /// a `Positive` instance allows immediate pruning of other branches.
    Positive,

    /// Static fact absent from the initial state (Always False).
    ///
    /// If a component of a conjunction (AND) is `Negative`, the entire expression
    /// simplifies to `False`. In a universal quantifier ($\forall$), encountering
    /// a `Negative` instance allows immediate pruning of the expression.
    Negative,

    /// Dynamic fact that can be changed by action effects.
    ///
    /// Fluents cannot be pruned during the initial expansion phase based on the
    /// initial state alone, as their truth value changes over time. They are
    /// passed to the Datalog grounding engine for reachability analysis.
    Fluent,
}

impl fmt::Display for Inertia {
    /// Formats the [`Inertia`] variant into an uppercase string.
    ///
    /// This is used for both technical logging and the aligned table rendering
    /// in [`InertiaTable`].
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write into.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Inertia::Positive => "POSITIVE",
            Inertia::Negative => "NEGATIVE",
            Inertia::Fluent   => "FLUENT",
        };
        write!(f, "{}", label)
    }
}
