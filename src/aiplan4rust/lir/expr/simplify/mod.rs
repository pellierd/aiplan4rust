pub mod and_or;
pub mod quantifier;
pub mod not;
mod arithmetic;
mod comparison;
mod assign;
mod when;
pub mod simplify;

pub use simplify::simplify;
