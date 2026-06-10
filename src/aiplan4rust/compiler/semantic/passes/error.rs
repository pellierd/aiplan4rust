use crate::aiplan4rust::compiler::semantic::signature_checker::SignatureMatcherError;
use crate::aiplan4rust::compiler::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::compiler::semantic::type_checker::TypeCheckerError;
use crate::aiplan4rust::error::Traceable;
use thiserror::Error;

/// Unified error type for semantic analysis passes.
///
/// This enum aggregates errors from various sub-systems, such as the
/// [`SignatureMatcher`] and the [`TypeChecker`]. It serves as the primary
/// error return type for all functions within the `passes` module.
///
/// By using `#[error(transparent)]`, this type delegates its `Display`
/// and `source` implementations to the underlying error types, ensuring
/// that diagnostic messages remain precise and informative.
#[derive(Debug, Error)]
pub enum SemanticPassError {
    /// Errors occurring during symbol signature matching (e.g., arity mismatch).
    #[error(transparent)]
    SignatureMatcher(#[from] SignatureMatcherError),

    /// Errors occurring during type hierarchy or compatibility checks.
    #[error(transparent)]
    TypeChecker(#[from] TypeCheckerError),

    #[error(transparent)]
    SymbolTable(#[from] SymbolTableError),

    // La nouvelle erreur ajoutée pour la capacité
    #[error("Union type exceeds maximum simplification capacity of {max} members (found {found})")]
    UnionCapacityExceeded { max: usize, found: usize },
}

impl SemanticPassError {
    /// Helper constructor to create a `UnionCapacityExceeded` error variant.
    pub fn union_capacity_exceeded(max: usize, found: usize) -> Self {
        Self::UnionCapacityExceeded { max, found }.trace()
    }
}

impl Traceable for SemanticPassError {}
