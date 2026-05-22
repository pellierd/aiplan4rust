use crate::aiplan4rust::lir::old::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::old::expr::ExprError;
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
