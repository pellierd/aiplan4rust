use thiserror::Error;

/// Erreurs spécifiques à la gestion de l'arène (`Arena`).
#[derive(Error, Debug)]
pub enum ArenaError {
    #[error("Node with id {0} not found in the arena")]
    NodeNotFound(usize),

    #[error("Root ID is missing in the arena")]
    MissingRootId,

    #[error("Root ID {id} is out of bounds (expected between 0 and {max})")]
    NodeIdOutOfBounds {
        id: usize,
        max: usize,
    },

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl ArenaError {
    pub fn node_not_found(id: usize) -> Self {
        ArenaError::NodeNotFound(id)
    }

    pub fn missing_root_id() -> Self {
        ArenaError::MissingRootId
    }

    pub fn node_id_out_of_bounds(id: usize, max: usize) -> Self {
        ArenaError::NodeIdOutOfBounds { id, max }
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        ArenaError::InternalError(message.into())
    }
}
