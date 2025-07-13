pub mod validation;
pub mod error;
pub mod checks;

pub type WellFormedError = ValidationError;

pub use error::ValidationError;

pub use self::validation::{check_well_formed, is_well_formed};
