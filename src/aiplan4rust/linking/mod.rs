//! This module organizes the linking phase of the aiplan4rust compilation pipeline.
//!
//! It provides functionality for semantic linking between a domain and a problem,
//! ensuring that identifiers and symbols are resolved and verified correctly.
//!
//! # Submodules
//!
//! - `context`: Defines the `LinkedSemanticContext` which holds the combined ASTs, symbol tables,
//!   and interner after linking.
//! - `linker`: Implements the `Linker` responsible for performing the linking process and running checks.
//! - `linker_result`: Contains the `LinkerResult` struct that encapsulates the output of the linking phase,
//!   including diagnostics.
//! - `checks`: Internal module that performs various semantic and structural validation checks during linking.
//! - `error`: Defines the `LinkingError` enum used for error handling within the linking phase.
//!
//! # Re-exports
//!
//! To simplification usage, the following types are re-exported:
//! - `LinkedSemanticContext` from the `context` module.
//! - `Linker` from the `linker` module.
//! - `LinkerResult` from the `linker_result` module.
//! - `LinkingError` from the `error` module.
//!
//! This organization facilitates a clean API for linking domain and problem contexts
//! with proper error and diagnostic handling.
pub mod context;
pub mod linker;
pub mod result;
mod checks;
pub mod error;

pub use context::LinkedSemanticContext;
pub use linker::Linker;
pub use result::Result as LinkerResult;
pub use error::LinkingError;
