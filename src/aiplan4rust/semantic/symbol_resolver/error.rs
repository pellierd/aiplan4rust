use crate::aiplan4rust::semantic::signature_matcher::SignatureMatcherError;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum SymbolResolverError {
    #[error(transparent)]
    SignatureMatcher(#[from] SignatureMatcherError),
}
