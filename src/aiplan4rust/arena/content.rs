use std::collections::HashMap;
use std::fmt::Debug;
use ordered_float::OrderedFloat;
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::interner::InternerDisplay;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, Ident, Optimization};

/// Trait representing the semantic content stored within a arena syntax.
///
/// This trait abstracts over the possible types of content a syntax can carry,
/// such as identifiers, numeric literals, operators, and optimization directives.
/// It provides methods to query the content as various expected types and
/// convenience methods to attempt conversions with error handling.
///
/// # Provided Methods
///
/// - `as_*` methods: Attempt to view the content as a specific type, returning
///   `Option<T>`. If the content matches the queried type, returns `Some(value)`,
///   otherwise `None`.
///
/// - `try_*` methods: Attempt to extract the content as a specific type, returning
///   a `Result<T, ParserInternalError>`. If the content does not match, returns
///   an error with a descriptive message.
///
/// - `is_none`: Returns whether the content is semantically empty or none. Defaults
///   to `false` and can be overridden.
///
/// # Typical Usage
///
/// Implementors of this trait represent concrete syntax content types in the
/// syntax arena. Parsing or semantic analysis code can use `try_*` methods to
/// safely extract strongly typed information from nodes, handling errors gracefully.
///
/// # Example
///
/// ```rust
/// use ordered_float::OrderedFloat;
/// use crate::aiplan4rust::frontend::ParserInternalError;
/// use crate::aiplan4rust::syntax::elements::{Ident, ArithmeticOp};
///
/// struct MyContent {
///     kind: MyContentKind,
///     ident: Option<Ident>,
///     float_val: Option<OrderedFloat<f64>>,
///     // other fields...
/// }
///
/// impl NodeContent for MyContent {
///     fn as_ident(&self) -> Option<Ident> {
///         self.ident.clone()
///     }
///
///     fn as_float(&self) -> Option<OrderedFloat<f64>> {
///         self.float_val
///     }
///
///     // implement other as_* methods as needed...
/// }
/// ```
///
/// # Notes
///
/// - The trait separates “view” methods (`as_*`) from fallible “try” methods (`try_*`)
///   to allow flexible and explicit error handling during parsing or semantic
///   processing.
/// - The default `try_*` implementations rely on the corresponding `as_*`
///   method and convert `None` to an error.
///
/// # See Also
///
/// - [`AiplanError`] for error type used in `try_*` methods.
/// - [`Ident`], [`ArithmeticOp`], [`AssignOp`], [`BinaryComp`], [`Optimization`] types
///   representing common semantic elements.
///
pub trait NodeContent : Clone + Debug {

    /// Returns `true` if the content is semantically empty or none.
    ///
    /// Defaults to `false`. Can be overridden to signal absence of content.
    fn is_none(&self) -> bool { false }
}
