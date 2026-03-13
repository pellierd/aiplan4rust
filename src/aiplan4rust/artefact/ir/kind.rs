//! Semantic classification of Intermediate Representation (IR) artifacts.
//!
//! This module defines the [`IRKind`] enum, which categorizes serialized planning artifacts
//! used in the planning pipeline. Each variant represents a distinct stage or form of
//! planning content, enabling the pipeline to process IR files correctly and safely.
//!
//! # IRKind Variants
//!
//! - `ParsedDomain`: Fully parsed planning domain, including types, predicates, and actions.
//! - `ParsedProblem`: Fully parsed problem file, including objects, initial state, and goals.
//! - `LiftedProblem`: Lifted problem derived from a parsed problem, suitable for reasoning
//!   in lifted space.
//!
//! # Serialization
//!
//! The enum derives [`Serialize`] and [`Deserialize`] for Serde-compatible formats
//! (JSON, CBOR, binary, etc.). These values are stable and safe to serialize/deserialize
//! across versions of the application.
//!
//! # Usage Example
//!
//! ```rust,ignore
//! use aiplan4rust::artefact::ir::IRKind;
//!
//! let kind = IRKind::ParsedDomain;
//! match kind {
//!     IRKind::ParsedDomain => println!("This is a domain"),
//!     IRKind::ParsedProblem => println!("This is a problem"),
//!     IRKind::LiftedProblem => println!("This is a lifted problem"),
//! }
//! ```
//!
//! # Display Formatting
//!
//! The `Display` implementation provides a human-readable string representation:
//!
//! ```rust,ignore
//! use aiplan4rust::artefact::ir::IRKind;
//!
//! let kind = IRKind::LiftedProblem;
//! println!("{}", kind); // Prints "Lifted Problem"
//! ```
//!
//! # Integration with the Pipeline
//!
//! `IRKind` is typically used to select appropriate processing functions
//! (parsing, linking, analysis) depending on the either_type of artifact. For example,
//! a `ParsedDomain` may be linked with a problem, while a `LiftedProblem` may
//! be used directly in reasoning or planning algorithms.

use std::fmt;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};

/// Semantic classification of an intermediate representation (IR) artifact.
///
/// Indicates the either_type and stage of a serialized planning file, enabling correct
/// processing within the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IRKind {
    /// Fully parsed domain artifact.
    ParsedDomain,

    /// Fully parsed problem artifact.
    ParsedProblem,

    /// Lifted problem artifact.
    LiftedProblem,

    GroundedProblem,
}

impl Display for IRKind {
    /// Formats the IR kind as a human-readable string.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use aiplan4rust::artefact::ir::IRKind;
    ///
    /// let kind = IRKind::LiftedProblem;
    /// println!("{}", kind); // Prints "Lifted Problem"
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            IRKind::ParsedDomain => "Domain",
            IRKind::ParsedProblem => "Problem",
            IRKind::LiftedProblem => "Lifted Problem",
            IRKind::GroundedProblem => "Grounded Problem",
        };
        write!(f, "{}", s)
    }
}
