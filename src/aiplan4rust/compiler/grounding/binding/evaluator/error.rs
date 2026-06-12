use std::fmt::{Debug, Display};

/// Trait d'effacement de type pour toutes les erreurs d'évaluation d'expressions.
pub trait ExprEvaluatorError: Debug + Display + std::error::Error + Send + Sync {}

/// Implémentation générique permettant à l'opérateur `?` de convertir automatiquement
/// n'importe quelle erreur concrète valide en un `Box<dyn ExprEvaluatorError>`.
impl<T> From<T> for Box<dyn ExprEvaluatorError>
where
    T: ExprEvaluatorError + 'static,
{
    fn from(err: T) -> Self {
        Box::new(err)
    }
}
