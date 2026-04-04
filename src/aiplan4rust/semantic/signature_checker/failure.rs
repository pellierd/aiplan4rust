//! Diagnostic Metadata for Signature Mismatches
//!
//! This module defines the [`MatchFailure`] enum, which captures the specific
//! reason why a symbol usage fails to match its declaration.
//!
//! It follows an **Expected vs. Observed** pattern across all variants, ensuring
//! that the semantic engine provides clear and actionable feedback for
//! structural, identity, and type-based errors.

use crate::aiplan4rust::lang::{SymbolId, Type};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the specific cause of a failure during the signature matching process.
///
/// Each variant provides the necessary context to generate detailed error messages,
/// pinpointing exactly where the usage site deviates from the declaration.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum MatchFailure {
    /// The resolved symbol identifier does not match the expected one.
    Symbol {
        /// The symbol identifier required by the declaration.
        expected: SymbolId,
        /// The actual symbol identifier found at the usage site.
        observed: SymbolId,
    },

    /// The number of arguments provided does not match the declaration's requirements.
    Arity {
        /// The number of parameters defined in the signature.
        expected: usize,
        /// The actual number of arguments provided at the call site.
        observed: usize,
    },

    /// The category of the symbol (e.g., Action vs Predicate) is incompatible
    /// with the usage context.
    KindMismatch,

    /// A specific argument failed the type compatibility check.
    Argument {
        /// The positional index of the failing argument (0-based).
        index: usize,
        /// The type required by the declaration's signature.
        expected: Type<SymbolId>,
        /// The actual type resolved from the provided argument.
        provided: Type<SymbolId>,
    },
}

impl fmt::Display for MatchFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Symbol { expected, observed } => {
                write!(
                    f,
                    "symbol mismatch: expected {expected:?}, got {observed:?}"
                )
            }
            Self::Arity { expected, observed } => {
                write!(
                    f,
                    "arity mismatch: expected {expected} args, got {observed}"
                )
            }
            Self::KindMismatch => {
                write!(f, "kind mismatch: incompatible symbol categories")
            }
            Self::Argument {
                index,
                expected,
                provided,
            } => {
                write!(f, "type mismatch at arg[{index}]: {expected} vs {provided}")
            }
        }
    }
}
