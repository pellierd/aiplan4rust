use thiserror::Error;
use crate::aiplan4rust::grounding::problem::index_table::IndexTableError;
use crate::aiplan4rust::interner::InternerError;

#[derive(Debug, Error)]
pub enum GroundingError {

    #[error(transparent)]
    Interner(#[from] InternerError),

    #[error(transparent)]
    IndexTable(#[from] IndexTableError),

}

impl GroundingError {}
