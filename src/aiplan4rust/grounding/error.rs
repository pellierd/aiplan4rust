use thiserror::Error;
use crate::aiplan4rust::grounding::problem::index_table::IndexTableError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::flatten_types::TypeFlattenError;
use crate::aiplan4rust::lir::LirError;

#[derive(Debug, Error)]
pub enum GroundingError {

    #[error(transparent)]
    Interner(#[from] InternerError),

    #[error(transparent)]
    IndexTable(#[from] IndexTableError),

    #[error(transparent)]
    Lir(#[from] LirError),

    #[error(transparent)]
    TypeFlatten(#[from] TypeFlattenError),
}

impl GroundingError {}
