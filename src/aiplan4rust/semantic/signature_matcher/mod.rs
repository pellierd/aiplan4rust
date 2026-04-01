//! Signature Matching and Semantic Validation Engine
//!
//! This module provides the tools necessary to validate that symbol usages
//! conform to their declarations. It handles structural verification,
//! hierarchical symbol resolution, and advanced type checking (including upcasting).
//!
//! ## Sub-modules
//! * [`matcher`]: The core logic for checking signatures.
//! * [`result`]: Defines the outcome of a match (Success, Upcast, or Failure).
//! * [`failure`]: Detailed diagnostic information for mismatches.
//! * [`error`]: Internal and propagation errors for the matching process.

mod error;
mod failure;
pub mod matcher;
pub mod result;

// Re-exporting primary types for a cleaner public API
pub use error::SignatureMatcherError;
pub use failure::MatchFailure;
pub use matcher::SignatureMatcher;
pub use result::MatchResult;
