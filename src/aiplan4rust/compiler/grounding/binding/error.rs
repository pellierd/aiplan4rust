use crate::aiplan4rust::compiler::lir::expr::builder::ExprBuilderError;

use crate::aiplan4rust::compiler::syntax::ast::tree::error::SyntaxTreeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BindingError {
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    ExprBuilder(#[from] ExprBuilderError),
}

impl BindingError {}
