pub mod factoring {
    //! Factorisation et flattening des expressions.
    //!
    //! Réduction de la taille et aplatissage des AND/OR imbriqués.

    use crate::aiplan4rust::lir::expr::Expr;

    /// Factorise les parties communes des expressions.
    /// Exemple: `(A ∧ B) ∨ (A ∧ C) -> A ∧ (B ∨ C)`.
    pub fn factor_expression(expr: &mut Expr) {
        // TODO: implémenter factorisation
    }

    /// Aplatit les AND et OR imbriqués pour simplifier la structure finale.
    pub fn flatten_expression(expr: &mut Expr) {
        // TODO: implémenter flattening récursif
    }
}
