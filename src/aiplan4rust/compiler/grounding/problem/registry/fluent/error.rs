use crate::aiplan4rust::support::lang::id::{FluentId, NumericFluentId};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FluentRegistryError {
    /// Erreur lorsqu'un FluentID ne correspond à aucune entrée dans le pool des prédicats.
    #[error("Invalid FluentID: {id:?} (pool size: {pool_size})")]
    InvalidFluentID { id: FluentId, pool_size: usize },

    /// Erreur lorsqu'un NumericFluentID est introuvable.
    #[error("Invalid NumericFluentID: {id:?} (pool size: {pool_size})")]
    InvalidNumericFluentID {
        id: NumericFluentId,
        pool_size: usize,
    },
}

impl FluentRegistryError {
    /// Crée une erreur pour un FluentID (Prédicat) invalide.
    pub fn invalid_fluent_id(id: FluentId, pool_size: usize) -> Self {
        Self::InvalidFluentID { id, pool_size }
    }

    /// Crée une erreur pour un NumericFluentID invalide.
    pub fn invalid_numeric_fluent_id(id: NumericFluentId, pool_size: usize) -> Self {
        Self::InvalidNumericFluentID { id, pool_size }
    }
}
