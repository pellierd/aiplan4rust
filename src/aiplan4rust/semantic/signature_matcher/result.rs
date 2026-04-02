//! Outcome of the Signature Matching Process
//!
//! This module defines the [`MatchResult`] type, which encapsulates the result of
//! comparing a symbol usage against its declaration.
//!
//! ## Matching Philosophy
//! In a "Heavy Duty" planning context, a match isn't always binary (true/false).
//! This module supports three distinct semantic states:
//!
//! 1. **Exact Match**: The types are perfectly compatible via standard subtyping
//!    (e.g., passing a `Truck` where a `Vehicle` is expected).
//! 2. **Upcast Match**: The match is valid but requires a type promotion
//!    (Upcasting). This is specifically used in Bercher-style task matching where
//!    an Action can fulfill a Task.
//! 3. **No Match**: A structural or semantic violation occurred, documented by
//!    a detailed [`MatchFailure`].
//!
//! ## Integration
//! The [`MatchResult`] is designed to be consumed by the `SignatureChecker` to
//! determine if a call site is valid or if specific diagnostic warnings/errors
//! need to be emitted during the semantic analysis phase.

use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::semantic::signature_matcher::failure::MatchFailure;
use crate::aiplan4rust::semantic::symbol::Declaration;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the outcome of a signature matching operation.
///
/// This enum categorizes how a provided argument relates to its expected parameter,
/// ranging from a perfect structural match to a semantic upcast or a complete mismatch.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MatchResult {
    /// A perfect match where the provided type is a valid subtype of the expected type.
    Match,

    /// A successful match achieved through upcasting (e.g., Task-to-Action mapping).
    ///
    /// This variant stores the necessary metadata to perform late-bound diagnostics
    /// or to facilitate specific code generation/transformation for upcasted calls.
    UpcastMatch {
        /// The type required by the declaration's signature.
        expected: Type<SymbolId>,
        /// The actual type found at the usage site.
        provided: Type<SymbolId>,
        /// The declaration of the argument (Constant, Variable, etc.).
        arg_decl: Declaration,
        /// The unique identifier of the AST node for this argument.
        arg_node_id: NodeId,
    },

    /// No valid match could be established.
    ///
    /// Contains a [`MatchFailure`] describing the exact nature of the structural
    /// or semantic incompatibility.
    NoMatch(MatchFailure),
}

impl MatchResult {
    /// Retourne vrai si la résolution est un succès (parfait ou par héritage)
    pub fn is_match(&self) -> bool {
        matches!(self, MatchResult::Match | MatchResult::UpcastMatch { .. })
    }
}

impl fmt::Display for MatchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Match => write!(f, "Exact Match"),
            Self::UpcastMatch {
                expected, provided, ..
            } => {
                write!(
                    f,
                    "Upcast Match (expected {}, provided {})",
                    expected, provided
                )
            }
            Self::NoMatch(failure) => write!(f, "No Match: {}", failure),
        }
    }
}
