pub mod error;
pub mod checks;

pub type WellFormedError = ValidationError;
pub type NormalizeError = ValidationError;

pub use error::ValidationError;
