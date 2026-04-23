use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::lang::VariableId;
use crate::aiplan4rust::lir::store::error::StorerError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExprBuilderError {
    #[error("duplicate variable declaration detected: {0}")]
    DuplicateVariable(VariableId),

    #[error("store error: {0}")]
    Store(#[from] StorerError),
}

impl ExprBuilderError {
    /// Creates a `DuplicateVariable` error and captures the call site.
    #[track_caller]
    pub fn duplicate_variable(var: VariableId) -> Self {
        ExprBuilderError::DuplicateVariable(var).trace()
    }
}

impl Traceable for ExprBuilderError {}
