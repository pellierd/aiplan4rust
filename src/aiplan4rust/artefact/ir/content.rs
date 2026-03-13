//! # IR Content Module
//!
//! This module defines the `IRContent` enum, which represents the actual content
//! of intermediate representation (IR) artifacts used in the planning pipeline.
//!
//! Each `IRContent` variant corresponds directly to an `IRKind` and stores the
//! associated content along with its serialization format (`SerdeFormat`). This
//! design ensures either_type-safe access to the underlying data while preserving format
//! information for serialization and deserialization.
//!
//! ## IRContent Variants
//!
//! - `ParsedDomain(SemanticContext, SerdeFormat)`
//!   - Represents a fully parsed planning domain.
//!   - Stores the semantic context and serialization format.
//!
//! - `ParsedProblem(SemanticContext, SerdeFormat)`
//!   - Represents a fully parsed planning problem.
//!   - Stores the semantic context and serialization format.
//!
//! - `LiftedProblem(LiftedProblem, SerdeFormat)`
//!   - Represents a lifted problem, suitable for lifted-space reasoning.
//!   - Stores the lifted problem object and serialization format.
//!
//! ## Serialization & Deserialization
//!
//! The module provides methods to serialize an `IRContent` instance to bytes
//! (`serialize_to_bytes`) and to deserialize from bytes (`deserialize_from_bytes`)
//! based on the `IRKind` and format. This enables seamless persistence and
//! transmission of IR artifacts.
//!
//! ## Accessing Inner Content
//!
//! The `inner()` method provides access to the actual underlying data of an `IRContent`:
//! - Returns a reference to `SemanticContext` for `ParsedDomain` and `ParsedProblem`.
//! - Returns a reference to `LiftedProblem` for `LiftedProblem`.
//!
//! The `IRContentInner` enum is used to safely wrap these references for ergonomic access.
//!
//! ## Example
//!
//! ```rust,ignore
//! use aiplan4rust::artefact::ir::content::{IRContent, IRContentInner};
//! use aiplan4rust::artefact::ir::kind::IRKind;
//! use aiplan4rust::semantic::SemanticContext;
//! use aiplan4rust::lir::problem::LiftedProblem;
//! use aiplan4rust::serialization::SerdeFormat;
//!
//! // Creating a parsed domain IR content
//! let domain_ctx = SemanticContext::default();
//! let content = IRContent::ParsedDomain(domain_ctx, SerdeFormat::Json);
//!
//! // Access the IR kind
//! assert_eq!(content.kind(), IRKind::ParsedDomain);
//!
//! // Serialize to bytes
//! let bytes = content.serialize_to_bytes().unwrap();
//!
//! // Deserialize from bytes
//! let restored = IRContent::deserialize_from_bytes(&bytes, IRKind::ParsedDomain, SerdeFormat::Json).unwrap();
//!
//! // Access inner content
//! match restored.inner() {
//!     IRContentInner::SemanticContext(ctx) => println!("SemanticContext length: {}", ctx.len()),
//!     IRContentInner::LiftedProblem(_) => println!("Lifted problem"),
//! }
//! ```

use std::fmt;
use crate::aiplan4rust::artefact::ir::kind::IRKind;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::grounding::problem::Problem;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::serialization::{SerdeFormat, SerdeSerializable, SerializationError};
use serde::{Deserialize, Serialize};


/// Enum representing the actual content of an IR artifact.
/// Each variant corresponds directly to an `IRKind` and stores the format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IRContent {
    ParsedDomain(SemanticContext, SerdeFormat),
    ParsedProblem(SemanticContext, SerdeFormat),
    LiftedProblem(LiftedProblem, SerdeFormat),
    GroundedProblem(Problem, SerdeFormat),
}

impl IRContent {
    /// Returns the `IRKind` corresponding to this content.
    ///
    /// # Parameters
    /// - `&self`: Reference to the current `IRContent`.
    ///
    /// # Returns
    /// - `IRKind`: The semantic either_type of this IR content (`ParsedDomain`, `ParsedProblem`, or `LiftedProblem`).
    pub fn kind(&self) -> IRKind {
        match self {
            IRContent::ParsedDomain(_, _) => IRKind::ParsedDomain,
            IRContent::ParsedProblem(_, _) => IRKind::ParsedProblem,
            IRContent::LiftedProblem(_, _) => IRKind::LiftedProblem,
            IRContent::GroundedProblem(_, _) => IRKind::GroundedProblem,
        }
    }

    /// Returns the stored serialization format.
    ///
    /// # Parameters
    /// - `&self`: Reference to the current `IRContent`.
    ///
    /// # Returns
    /// - `SerdeFormat`: The serialization format used to store this content.
    pub fn format(&self) -> SerdeFormat {
        match self {
            IRContent::ParsedDomain(_, fmt) => *fmt,
            IRContent::ParsedProblem(_, fmt) => *fmt,
            IRContent::LiftedProblem(_, fmt) => *fmt,
            IRContent::GroundedProblem(_, fmt) => *fmt,
        }
    }

    /// Serializes the IR content to a byte vector according to its stored format.
    ///
    /// # Parameters
    /// - `&self`: Reference to the current `IRContent`.
    ///
    /// # Returns
    /// - `Result<Vec<u8>, SerializationError>`: Serialized bytes on success, or an error if serialization fails.
    pub fn serialize_to_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        match self {
            IRContent::ParsedDomain(inner, fmt) | IRContent::ParsedProblem(inner, fmt) => {
                inner.serialize_to_bytes(*fmt)
            }
            IRContent::LiftedProblem(inner, fmt) => inner.serialize_to_bytes(*fmt),
            IRContent::GroundedProblem(inner, fmt) => inner.serialize_to_bytes(*fmt),
        }
    }

    /// Deserializes IR content from a byte slice using the specified kind and format.
    ///
    /// # Parameters
    /// - `bytes`: Slice of bytes containing serialized IR content.
    /// - `kind`: The expected `IRKind` of the content.
    /// - `format`: The `SerdeFormat` used for deserialization.
    ///
    /// # Returns
    /// - `Result<IRContent, SerializationError>`: Deserialized `IRContent` on success, or an error if deserialization fails.
    pub fn deserialize_from_bytes(
        bytes: &[u8],
        kind: IRKind,
        format: SerdeFormat,
    ) -> Result<Self, SerializationError> {
        let content = match kind {
            IRKind::ParsedDomain => IRContent::ParsedDomain(
                SemanticContext::deserialize_from_bytes(bytes, format)?,
                format,
            ),
            IRKind::ParsedProblem => IRContent::ParsedProblem(
                SemanticContext::deserialize_from_bytes(bytes, format)?,
                format,
            ),
            IRKind::LiftedProblem => IRContent::LiftedProblem(
                LiftedProblem::deserialize_from_bytes(bytes, format)?,
                format,
            ),
            IRKind::GroundedProblem => IRContent::GroundedProblem(
                Problem::deserialize_from_bytes(bytes, format)?,
                format,
            ),
        };
        Ok(content)
    }

    /// Returns a reference to the inner content of the IR.
    ///
    /// # Parameters
    /// - `&self`: Reference to the current `IRContent`.
    ///
    /// # Returns
    /// - `IRContentInner<'_>`: A reference wrapper to the inner object:
    ///   - `SemanticContext` for `ParsedDomain` and `ParsedProblem`.
    ///   - `LiftedProblem` for `LiftedProblem`.
    pub fn inner(&self) -> IRContentInner<'_> {
        match self {
            IRContent::ParsedDomain(inner, _) | IRContent::ParsedProblem(inner, _) => {
                IRContentInner::SemanticContext(inner)
            }
            IRContent::LiftedProblem(inner, _) => IRContentInner::LiftedProblem(inner),
            IRContent::GroundedProblem(inner, _) => IRContentInner::GroundedProblem(inner),
        }
    }
}

/// Provides access to the inner content of an IR artifact.
///
/// This enum is returned by [`IRContent::inner()`] and allows read-only access
/// to the actual planning structures.
#[derive(Debug, Clone)]
pub enum IRContentInner<'a> {
    /// Reference to a `SemanticContext` (used in `ParsedDomain` and `ParsedProblem`).
    SemanticContext(&'a SemanticContext),

    /// Reference to a `LiftedProblem` (used in `LiftedProblem`).
    LiftedProblem(&'a LiftedProblem),

    /// Reference to a `LiftedProblem` (used in `LiftedProblem`).
    GroundedProblem(&'a Problem),
}

impl<'a> fmt::Display for IRContentInner<'a> {
    /// Delegates `Display` to the inner content.
    ///
    /// This will print the full content of the `SemanticContext` or `LiftedProblem`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IRContentInner::SemanticContext(ctx) => write!(f, "{}", ctx),
            IRContentInner::LiftedProblem(pb) => write!(f, "{}", pb),
            IRContentInner::GroundedProblem(pb) => write!(f, "{}", pb),
        }
    }
}
