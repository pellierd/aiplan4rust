use crate::aiplan4rust::lir::expr::Expr;
pub mod temporal {
    //! Transformations spécifiques aux opérateurs temporels PDDL.
    //!
    //! Distribution des opérateurs temporels (`always`, `at start`, `at end`) sur les littéraux et AND/OR.

    use crate::aiplan4rust::lir::expr::Expr;

    /// Normalise les opérateurs temporels dans une expression.
    pub fn normalize_temporal(expr: &mut Expr) {
        // TODO: implémenter la distribution des opérateurs temporels
    }
}
