use thiserror::Error;
use crate::aiplan4rust::interner::InternerError;

#[derive(Debug, Error)]
pub enum GroundingError {

    #[error(transparent)]
    Interner(#[from] InternerError),

}

impl GroundingError {}
