use crate::aiplan4rust::semantic::signature_matcher::SignatureMatcherError;
use crate::aiplan4rust::semantic::type_checker::TypeCheckerError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SymbolResolverError {
    #[error(transparent)]
    SignatureMatcher(#[from] SignatureMatcherError),

    #[error(transparent)]
    TypeChecker(#[from] TypeCheckerError),
}
