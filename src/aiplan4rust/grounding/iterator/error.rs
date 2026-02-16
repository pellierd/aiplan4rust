use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainIteratorError {
    /// Le produit cartésien des domaines dépasse la capacité de usize.
    #[error("Explosion combinatoire (Arité {arity}): le nombre de combinaisons dépasse les limites du système (64-bit).")]
    CombinatorialExplosion {
        arity: usize,
    },

}

impl DomainIteratorError {
    /// Crée une nouvelle erreur d'explosion combinatoire (Overflow).
    #[track_caller]
    pub fn combinatorial_explosion(arity: usize) -> Self {
        Self::CombinatorialExplosion { arity }
    }
}
