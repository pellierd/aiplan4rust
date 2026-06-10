use crate::aiplan4rust::error::Traceable;

use crate::aiplan4rust::lir::expr::builder::ExprBuilderError;
use crate::aiplan4rust::lir::expr::error::StorerError;
use crate::aiplan4rust::lir::expr::ops::error::ExprOpError;
use crate::aiplan4rust::lir::expr::ExprId;
use crate::aiplan4rust::lir::problem::registry::IndexTableError;
use crate::aiplan4rust::lir::problem::LiftedProblemError;
use crate::aiplan4rust::support::interner::InternerError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NormalizationError {
    #[error(transparent)]
    Interner(#[from] InternerError),
    #[error(transparent)]
    LiftedProblem(#[from] LiftedProblemError),
    #[error(transparent)]
    IndexTable(#[from] IndexTableError),

    #[error(transparent)]
    ExprStore(#[from] StorerError),
    #[error(transparent)]
    ExpOpHC(#[from] ExprOpError),

    #[error(transparent)]
    ExpBuilder(#[from] ExprBuilderError),

    #[error(
        "Normalization error: expected expression ID {id:?} was not found in the scratchpad cache"
    )]
    MissingCachedExpression { id: ExprId },
}

impl NormalizationError {
    #[inline]
    pub fn missing_cache(id: ExprId) -> Self {
        Self::MissingCachedExpression { id }.trace()
    }
}

impl Traceable for NormalizationError {}
