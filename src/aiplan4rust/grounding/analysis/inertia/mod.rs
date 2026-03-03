pub mod table;
pub mod inertia;

pub mod registry;
pub mod error;

pub use error::InertiaError;
pub use table::InertiaTable;

pub use table::builder::build;
