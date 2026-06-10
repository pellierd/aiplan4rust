//! Domain Constants Encoding
//!
//! This module handles the extraction of constants from the domain file.
//! It reuses the generic object encoding ops and marks the constant boundary
//! in the LIR to ensure global symbol resolution consistency.

use crate::aiplan4rust::compiler::lir::encoding::{objects_def, EncodingError, EncodingRegistry};
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::compiler::syntax::ast::tree::SyntaxSubtree;
use crate::aiplan4rust::compiler::syntax::ast::AstNode;

/// Encodes domain constants and defines the global constant offset.
///
/// This function delegates the parsing of typed symbols to `objects_def::encoding`.
/// After the constants are registered, it triggers `ir.set_constant_offset()`
/// to finalize the domain-level symbol space before any problem-level objects
/// are added.
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `:constants` block.
/// * `evaluator` - The mutable encoding context for symbol-to-ID mapping.
/// * `ir` - The mutable Lifted Problem where constants are stored.
pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), EncodingError> {
    objects_def::encode(subtree, registry, ir)?;
    Ok(())
}
