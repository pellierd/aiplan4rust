pub mod grounder;
pub mod error;
mod result;
pub mod problem;

pub use result::Result as GroundingResult;
pub use grounder::Grounder;
pub use problem::Problem;
