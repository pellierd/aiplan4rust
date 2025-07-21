use std::collections::HashMap;
use std::fmt::Debug;
use ordered_float::OrderedFloat;
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::interner::InternerDisplay;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, BinaryComp, Ident, Optimization};

pub trait SyntaxContent:  InternerDisplay + Clone + Debug {
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

    /// Applies identifier remapping to the content of the syntax using the provided map.
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
    fn try_ident(&self) -> Result<Ident, AiplanError> {
        self.as_ident()
            .ok_or_else(|| AiplanError::InternalError("Not an Ident".to_string()))
    }

    /// Attempts to extract a floating-point value from the content.
    ///
    /// Returns `Ok(OrderedFloat<f64>)` if successful or
    /// `Err(ParserInternalError)` if the content is not a float.
    fn try_float(&self) -> Result<OrderedFloat<f64>, AiplanError> {
        self.as_float()
            .ok_or_else(|| AiplanError::InternalError("Not a Float".to_string()))
    }

    /// Attempts to extract a binary comparison operator from the content.
    ///
    /// Returns `Ok(BinaryComp)` if successful or
    /// `Err(ParserInternalError)` if the content is not a binary comparison.
    fn try_binary_comp(&self) -> Result<BinaryComp, AiplanError> {
        self.as_binary_comp()
            .ok_or_else(|| AiplanError::InternalError("Not a BinaryComp".to_string()))
    }

    /// Attempts to extract an assignment operator from the content.
    ///
    /// Returns `Ok(AssignOp)` if successful or
    /// `Err(ParserInternalError)` if the content is not an assignment operator.
    fn try_assign_op(&self) -> Result<AssignOp, AiplanError> {
        self.as_assign_op()
            .ok_or_else(|| AiplanError::InternalError("Not an AssignOp".to_string()))
    }

    /// Attempts to extract an arithmetic operator from the content.
    ///
    /// Returns `Ok(ArithmeticOp)` if successful or
    /// `Err(ParserInternalError)` if the content is not an arithmetic operator.
    fn try_arithmetic_op(&self) -> Result<ArithmeticOp, AiplanError> {
        self.as_arithmetic_op()
            .ok_or_else(|| AiplanError::InternalError("Not an ArithmeticOp".to_string()))
    }

    /// Attempts to extract an optimization directive from the content.
    ///
    /// Returns `Ok(Optimization)` if successful or
    /// `Err(ParserInternalError)` if the content is not an optimization.
    fn try_optimization(&self) -> Result<Optimization, AiplanError> {
        self.as_optimization()
            .ok_or_else(|| AiplanError::InternalError("Not an Optimization".to_string()))
    }
}
