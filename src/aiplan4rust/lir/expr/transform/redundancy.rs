use crate::aiplan4rust::lir::expr::Expr;
pub mod redundancy {
    //! Élimination des redondances dans les expressions.
    //!
    //! Supprime les doublons, tautologies et contradictions après CNF/DNF ou NNF.

    use crate::aiplan4rust::lir::expr::Expr;

    /// Supprime doublons, contradictions (`A ∧ ¬A -> false`) et tautologies (`A ∨ ¬A -> true`)
    pub fn eliminate_redundancy(expr: &mut Expr) {
        // TODO: implémenter la suppression des doublons, tautologies et contradictions
    }
}
