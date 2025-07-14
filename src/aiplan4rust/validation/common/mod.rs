pub mod error;
pub mod checks;

pub type WellFormedError = ValidationError;
pub type WellNormalizedError = ValidationError;

pub use error::ValidationError;
