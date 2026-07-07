use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;

/// Explique l'origine d'un effet pour le grounding.
/// On dérive Copy car c'est un type "POD" (Plain Old Data) léger.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Cause {
    /// L'effet est déclenché systématiquement par l'action.
    Action,
    /// L'effet dépend d'une condition (When), représentée par ce pivot.
    Pivot(Atom),
}
