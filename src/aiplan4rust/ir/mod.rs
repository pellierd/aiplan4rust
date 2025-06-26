use crate::aiplan4rust::ir::expr::Expr;

pub mod builder;
pub mod expr;

pub use builder::IRBuilder;


pub type Precondition = Expr;
pub type Effect = Expr;
