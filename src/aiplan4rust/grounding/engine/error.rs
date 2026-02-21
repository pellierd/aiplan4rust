use thiserror::Error;
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::logic::LogicError;
use crate::aiplan4rust::tree::error::SyntaxTreeError;

#[derive(Error, Debug)]
pub enum GroundingEngineError {

    #[error(transparent)]
    Logic(#[from] LogicError),

    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    Expr(#[from] ExprError),
}

impl GroundingEngineError {

}
