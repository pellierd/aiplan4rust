use thiserror::Error;
use crate::aiplan4rust::syntax::ast::{AstContent, AstKind};
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Errors specific to the `expr` module, primarily related to conversion failures.
///
/// This enum represents the various error conditions that can occur during
/// the parsing and conversion of syntax tree nodes into expressions.
///
/// # Variants
///
/// - [`SyntaxTree`]: Wraps errors originating from the underlying syntax tree system.
/// - [`UnsupportedContent`]: Indicates that an unexpected or unsupported `AstContent` variant was encountered.
/// - [`UnsupportedKind`]: Indicates that an unexpected or unsupported `AstKind` variant was encountered.
/// - [`InternalError`]: Represents generic internal errors for unexpected states or conditions.
///
/// # Usage
///
/// These errors are typically returned during the transformation from
/// syntax tree representations to intermediate expression forms, allowing
/// detailed reporting of unsupported or invalid constructs.
///
/// # Examples
///
/// ```
/// use aiplan4rust::lir::expr::ExprError;
/// use aiplan4rust::syntax::ast::{AstContent, AstKind};
///
/// let err = ExprError::unsupported_content(AstContent::Ident(42));
/// let err_kind = ExprError::unsupported_kind(AstKind::EffectDef);
/// let internal = ExprError::internal_error("Unexpected null value");
/// ```
#[derive(Error, Debug)]
pub enum ExprError {
    /// An error originating from the syntax tree system.
    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    /// Indicates that an unsupported or unexpected `AstContent` variant was encountered.
    #[error("Unsupported content: {content:?}")]
    UnsupportedContent {
        /// The unsupported AST content variant that triggered the error.
        content: AstContent,
    },

    /// Indicates that an unsupported or unexpected `AstKind` variant was encountered.
    #[error("Unsupported kind: {kind:?}")]
    UnsupportedKind {
        /// The unsupported AST kind variant that triggered the error.
        kind: AstKind,
    },

}

impl ExprError {
    /// Creates an `UnsupportedContent` error variant for the given `AstContent`.
    ///
    /// # Arguments
    ///
    /// * `content` - The unsupported AST content variant encountered.
    ///
    /// # Returns
    ///
    /// A new `ExprError` representing the unsupported content error.
    pub fn unsupported_content(content: AstContent) -> Self {
        ExprError::UnsupportedContent { content }
    }

    /// Creates an `UnsupportedKind` error variant for the given `AstKind`.
    ///
    /// # Arguments
    ///
    /// * `kind` - The unsupported AST kind variant encountered.
    ///
    /// # Returns
    ///
    /// A new `ExprError` representing the unsupported kind error.
    pub fn unsupported_kind(kind: AstKind) -> Self {
        ExprError::UnsupportedKind { kind }
    }

}
