//! Module for defining the `SyntaxContent` trait, which represents
//! the semantic content of syntax nodes in the syntax tree.
//!
//! This trait provides a uniform interface for extracting
//! various semantic types such as identifiers, literals,
//! operators, and directives from syntax node content.
//!
//! It also supports remapping identifiers within the content,
//! which is useful during syntax transformations or renaming phases.
//!
//! The trait requires implementors to support cloning, debugging,
//! and formatted display via the `InternerDisplay` trait, which
//! enables resolving interned strings.
//!
//! # Semantic Content Extraction
//!
//! Implementors provide methods to optionally retrieve content as:
//! - Identifiers (`Ident`)
//! - Floating-point literals (`OrderedFloat<f64>`)
//! - Binary comparison operators (`BinaryComp`)
//! - Assignment operators (`AssignOp`)
//! - Arithmetic operators (`ArithmeticOp`)
//! - Optimization directives (`Optimization`)
//!
//! Additionally, methods are provided to attempt extraction that return
//! detailed errors (`SyntaxTreeError`) when the expected content is absent or mismatched.
//!
//! # Identifier Remapping
//!
//! The `remap_idents` method allows updating identifiers according to a
//! provided mapping, facilitating tasks like renaming or symbol resolution.

use std::fmt::{Debug, Display};
use ordered_float::OrderedFloat;
use crate::aiplan4rust::lang::{ArithmeticOp, AssignOp, CompareOp, OptimizationOp};
use crate::aiplan4rust::tree::error::SyntaxTreeError;

/// Trait representing the semantic content of a syntax node.
///
/// Types implementing this trait can express the specific kind
/// of content they hold, such as identifiers, literals, or operators,
/// and provide methods for extracting these in a type-safe manner.
///
/// The trait also supports remapping of identifiers via a
/// supplied mapping, useful during syntax tree transformations.
///
/// # Requirements
///
/// Implementors must also implement:
/// - [`InternerDisplay`] to support pretty-printing with identifier interning.
/// - [`Clone`] for safe copying.
/// - [`Debug`] for debugging purposes.
pub trait SyntaxContent: Display + Clone + Debug  + Default {

    /// Returns the content as a floating-point number if available.
    fn as_number(&self) -> Option<OrderedFloat<f64>>;

    /// Returns the content as a binary comparison operator if available.
    fn as_compare_op(&self) -> Option<CompareOp>;

    /// Returns the content as an assignment operator if available.
    fn as_assign_op(&self) -> Option<AssignOp>;

    /// Returns the content as an arithmetic operator if available.
    fn as_arithmetic_op(&self) -> Option<ArithmeticOp>;

    /// Returns the content as an optimization directive if available.
    fn as_optimization_op(&self) -> Option<OptimizationOp>;

    /// Returns `true` if the content is semantically empty or none.
    ///
    /// Defaults to `false`. Can be overridden to signal absence of content.
    fn is_none(&self) -> bool { false }

    /// Attempts to extract a floating-point value from the content.
    ///
    /// Returns `Ok(OrderedFloat<f64>)` if successful or
    /// `Err(SyntaxTreeError::NotAFloat)` if the content is not a float.
    fn try_number(&self) -> Result<OrderedFloat<f64>, SyntaxTreeError> {
        self.as_number()
            .ok_or_else(|| SyntaxTreeError::not_a_float())
    }

    /// Attempts to extract a binary comparison operator from the content.
    ///
    /// Returns `Ok(BinaryComp)` if successful or
    /// `Err(SyntaxTreeError::NotABinaryComp)` if the content is not a binary comparison.
    fn try_compare_op(&self) -> Result<CompareOp, SyntaxTreeError> {
        self.as_compare_op()
            .ok_or_else(|| SyntaxTreeError::not_a_binary_comp())
    }

    /// Attempts to extract an assignment operator from the content.
    ///
    /// Returns `Ok(AssignOp)` if successful or
    /// `Err(SyntaxTreeError::NotAnAssignOp)` if the content is not an assignment operator.
    fn try_assign_op(&self) -> Result<AssignOp, SyntaxTreeError> {
        self.as_assign_op()
            .ok_or_else(|| SyntaxTreeError::not_an_assign_op())
    }

    /// Attempts to extract an arithmetic operator from the content.
    ///
    /// Returns `Ok(ArithmeticOp)` if successful or
    /// `Err(SyntaxTreeError::NotAnArithmeticOp)` if the content is not an arithmetic operator.
    fn try_arithmetic_op(&self) -> Result<ArithmeticOp, SyntaxTreeError> {
        self.as_arithmetic_op()
            .ok_or_else(|| SyntaxTreeError::not_an_arithmetic_op())
    }

    /// Attempts to extract an optimization directive from the content.
    ///
    /// Returns `Ok(Optimization)` if successful or
    /// `Err(SyntaxTreeError::NotAnOptimization)` if the content is not an optimization.
    fn try_optimization_op(&self) -> Result<OptimizationOp, SyntaxTreeError> {
        self.as_optimization_op()
            .ok_or_else(|| SyntaxTreeError::not_an_optimization())
    }
}
