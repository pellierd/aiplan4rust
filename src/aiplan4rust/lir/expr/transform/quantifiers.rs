use crate::aiplan4rust::lir::expr::Expr;
pub mod quantifiers {
    //! Normalisation et déplacement des quantificateurs.
    //!
    //! Fusion et réorganisation des quantificateurs pour une structure canonique.

    use crate::aiplan4rust::lir::expr::Expr;

    /// Normalise les quantificateurs dans une expression.
    /// Exemple: `(forall x (forall x P)) -> (forall x P)`.
    pub fn normalize_quantifiers(expr: &mut Expr) {
        // TODO: implémenter fusion, réorganisation et suppression des doublons
    }
}
