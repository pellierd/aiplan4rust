use crate::aiplan4rust::lir::store::expr_old::ops::ExprOpError;
use crate::aiplan4rust::lir::store::expr_old::ExprError;
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BindingError {
    #[error(transparent)]
    Logic(#[from] ExprOpError),

    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    Expr(#[from] ExprError),
}

impl BindingError {}
