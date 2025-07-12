use std::collections::HashMap;
use ordered_float::OrderedFloat;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::InternerDisplay;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, Ident, Optimization};

/// Trait representing the semantic content stored within a arena node.
///
/// This trait abstracts over the possible types of content a node can carry,
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
/// Implementors of this trait represent concrete node content types in the
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
/// - [`ParserInternalError`] for error type used in `try_*` methods.
/// - [`Ident`], [`ArithmeticOp`], [`AssignOp`], [`BinaryComp`], [`Optimization`] types
///   representing common semantic elements.
///
pub trait NodeContent : InternerDisplay {
    /// Returns the content as an identifier if available.
    fn as_ident(&self) -> Option<Ident>;

    /// Returns the content as a floating-point number if available.
    fn as_float(&self) -> Option<OrderedFloat<f64>>;

    /// Returns the content as a binary comparison operator if available.
    fn as_binary_comp(&self) -> Option<BinaryComp>;

    /// Returns the content as an assignment operator if available.
    fn as_assign_op(&self) -> Option<AssignOp>;

    /// Returns the content as an arithmetic operator if available.
    fn as_arithmetic_op(&self) -> Option<ArithmeticOp>;

    /// Returns the content as an optimization directive if available.
    fn as_optimization(&self) -> Option<Optimization>;

    /// Returns `true` if the content is semantically empty or none.
    ///
    /// Defaults to `false`. Can be overridden to signal absence of content.
    fn is_none(&self) -> bool { false }

    /// Applies identifier remapping to the content of the node using the provided map.
    ///
    /// This is a generic wrapper that delegates to the content's own remap_idents method.
    ///
    /// # Arguments
    /// * `map` - A mapping from old identifiers to new ones.
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>);

    /// Attempts to extract an identifier from the content.
    ///
    /// Returns `Ok(Ident)` if successful or
    /// `Err(ParserInternalError)` if the content is not an identifier.
    fn try_ident(&self) -> Result<Ident, ParserInternalError> {
        self.as_ident()
            .ok_or_else(|| ParserInternalError::new("Not an Ident".to_string()))
    }

    /// Attempts to extract a floating-point value from the content.
    ///
    /// Returns `Ok(OrderedFloat<f64>)` if successful or
    /// `Err(ParserInternalError)` if the content is not a float.
    fn try_float(&self) -> Result<OrderedFloat<f64>, ParserInternalError> {
        self.as_float()
            .ok_or_else(|| ParserInternalError::new("Not a Float".to_string()))
    }

    /// Attempts to extract a binary comparison operator from the content.
    ///
    /// Returns `Ok(BinaryComp)` if successful or
    /// `Err(ParserInternalError)` if the content is not a binary comparison.
    fn try_binary_comp(&self) -> Result<BinaryComp, ParserInternalError> {
        self.as_binary_comp()
            .ok_or_else(|| ParserInternalError::new("Not a BinaryComp".to_string()))
    }

    /// Attempts to extract an assignment operator from the content.
    ///
    /// Returns `Ok(AssignOp)` if successful or
    /// `Err(ParserInternalError)` if the content is not an assignment operator.
    fn try_assign_op(&self) -> Result<AssignOp, ParserInternalError> {
        self.as_assign_op()
            .ok_or_else(|| ParserInternalError::new("Not an AssignOp".to_string()))
    }

    /// Attempts to extract an arithmetic operator from the content.
    ///
    /// Returns `Ok(ArithmeticOp)` if successful or
    /// `Err(ParserInternalError)` if the content is not an arithmetic operator.
    fn try_arithmetic_op(&self) -> Result<ArithmeticOp, ParserInternalError> {
        self.as_arithmetic_op()
            .ok_or_else(|| ParserInternalError::new("Not an ArithmeticOp".to_string()))
    }

    /// Attempts to extract an optimization directive from the content.
    ///
    /// Returns `Ok(Optimization)` if successful or
    /// `Err(ParserInternalError)` if the content is not an optimization.
    fn try_optimization(&self) -> Result<Optimization, ParserInternalError> {
        self.as_optimization()
            .ok_or_else(|| ParserInternalError::new("Not an Optimization".to_string()))
    }

}
