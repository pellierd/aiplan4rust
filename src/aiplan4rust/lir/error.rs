use thiserror::Error;
use crate::aiplan4rust::core::arena::ArenaError;
use crate::aiplan4rust::lang::LangError;
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::syntax::ast::AstError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;


/// General error type for the `lir` module.
#[derive(Debug, Error)]
pub enum LirError {
    /// An error originating from the expression system.
    #[error(transparent)]
    Expr(#[from] ExprError),

    #[error(transparent)]
    Lang(#[from] LangError),

    #[error(transparent)]
    Ast(#[from] AstError),

    /// An error from the syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    Arena(#[from] ArenaError),

    /// The AST structure of the action is unsupported or invalid for conversion.
    #[error("Unsupported action structure: {0}")]
    UnsupportedAction(String),

    /// The AST structure of the task network is unsupported or invalid for conversion.
    #[error("Unsupported task network structure: {0}")]
    UnsupportedTaskNetwork(String),

    /// Generic internal error.
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl LirError {
    pub fn expr(err: ExprError) -> Self {
        LirError::Expr(err)
    }

    pub fn syntax_tree(err: SyntaxTreeError) -> Self {
        LirError::SyntaxTree(err)
    }

    pub fn unsupported_action(msg: impl Into<String>) -> Self {
        LirError::UnsupportedAction(msg.into())
    }

    pub fn unsupported_task_network(msg: impl Into<String>) -> Self {
        LirError::UnsupportedTaskNetwork(msg.into())
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        LirError::InternalError(msg.into())
    }
}
