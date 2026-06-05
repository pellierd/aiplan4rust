use crate::aiplan4rust::lir::expr::builder::ExprBuilderError;

use crate::aiplan4rust::tree::error::SyntaxTreeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BindingError {
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    ExprBuilder(#[from] ExprBuilderError),
}

impl BindingError {}
