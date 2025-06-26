use crate::aiplan4rust::ir::expression::Expr;
use crate::aiplan4rust::tree::TreeArena;
pub mod builder;
pub mod expression;

pub use builder::IRBuilder;


pub type Precondition = TreeArena<Expr>;
pub type Effect = TreeArena<Expr>;
