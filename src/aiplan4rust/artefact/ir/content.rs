use crate::aiplan4rust::artefact::ir::kind::IRKind;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::serialization::SerializationError;
use crate::aiplan4rust::serialization::{SerdeFormat, SerdeSerializable};
use serde::{Deserialize, Serialize};

/// Enum représentant le contenu réel d'une IR.
/// Chaque variant correspond exactement à un `IRKind` et stocke le format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IRContent {
    ParsedDomain(SemanticContext, SerdeFormat),
    ParsedProblem(SemanticContext, SerdeFormat),
    LiftedProblem(LiftedProblem, SerdeFormat),
}

impl IRContent {
    /// Retourne le `IRKind` correspondant au contenu
    pub fn kind(&self) -> IRKind {
        match self {
            IRContent::ParsedDomain(_, _) => IRKind::ParsedDomain,
            IRContent::ParsedProblem(_, _) => IRKind::ParsedProblem,
            IRContent::LiftedProblem(_, _) => IRKind::LiftedProblem,
        }
    }

    /// Retourne le `SerdeFormat` stocké dans le contenu
    pub fn format(&self) -> SerdeFormat {
        match self {
            IRContent::ParsedDomain(_, fmt) => *fmt,
            IRContent::ParsedProblem(_, fmt) => *fmt,
            IRContent::LiftedProblem(_, fmt) => *fmt,
        }
    }

    /// Sérialise le contenu en bytes selon le format stocké
    pub fn serialize_to_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        match self {
            IRContent::ParsedDomain(inner, fmt) | IRContent::ParsedProblem(inner, fmt) => {
                inner.serialize_to_bytes(*fmt)
            }
            IRContent::LiftedProblem(inner, fmt) => inner.serialize_to_bytes(*fmt),
        }
    }

    /// Désérialise le contenu depuis des bytes selon le `IRKind` et le format donné
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
        };
        Ok(content)
    }

    /// Retourne une référence vers le contenu interne (SemanticContext ou LiftedProblem)
    pub fn inner(&self) -> IRContentInner<'_> {
        match self {
            IRContent::ParsedDomain(inner, _) | IRContent::ParsedProblem(inner, _) => {
                IRContentInner::SemanticContext(inner)
            }
            IRContent::LiftedProblem(inner, _) => IRContentInner::LiftedProblem(inner),
        }
    }
}

/// Enum pour accéder au contenu réel d'une IR via `inner()`
#[derive(Debug, Clone)]
pub enum IRContentInner<'a> {
    SemanticContext(&'a SemanticContext),
    LiftedProblem(&'a LiftedProblem),
}
