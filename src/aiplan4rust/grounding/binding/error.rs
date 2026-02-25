use thiserror::Error;
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::tree::error::SyntaxTreeError;

#[derive(Error, Debug)]
pub enum BindingError {

    #[error(transparent)]
    Logic(#[from] ExprOpError),

    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    Expr(#[from] ExprError),
}

impl BindingError {

}
