use crate::aiplan4rust::lir::passes::{typing, logic};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::LirError;

/// Performs the complete normalization pipeline for a [`LiftedProblem`].
///
/// This function is the primary entry point for transforming a "raw" Lifted Intermediate
/// Representation (LIR) into a "canonical" form ready for the grounding process.
/// The transformation is executed in two distinct stages to decouple logical
/// simplification from structural type resolution.
///
/// # The Normalization Pipeline
///
/// The process follows a strict order to ensure data integrity and performance:
///
/// 1. **Logical Normalization ([`logic::normalize`]):**
///    - Rewrites logical connectors (e.g., eliminating `imply` in favor of `or` and `not`).
///    - Pushes negations down to atomic formulas (Negation Normal Form).
///    - Simplifies boolean expressions and arithmetic constants.
///    - *Goal:* Ensure the expression tree is semantically as simple as possible.
///
/// 2. **Structural Normalization ([`typing::normalize`]):**
///    - Scans all expressions for ad-hoc `Either` type signatures.
///    - Materializes these anonymous unions into formal, named types within the global
///      [`SymbolRegistry`].
///    - Updates all variable references and parameter lists to point to these new atomic IDs.
///    - *Goal:* Eliminate complex type polymorphism to simplify the Grounder's work.
///
/// # Errors
///
/// Returns a [`LirError`] if:
/// * An expression is malformed during logical rewriting.
/// * The type materialization phase fails to register a new anonymous type due to
///   naming collisions or registry inconsistencies.
///
/// # Examples
///
/// ```rust
/// use crate::aiplan4rust::lir::problem::LiftedProblem;
/// use crate::aiplan4rust::lir::passes::normalization;
///
/// # fn example(mut problem: LiftedProblem) -> Result<(), Box<dyn std::error::Error>> {
/// // Transform the raw encoded problem into a grounded-ready problem
/// normalization::normalize(&mut problem)?;
/// # Ok(())
/// # }
/// ```
pub fn normalize(problem: &mut LiftedProblem) -> Result<(), LirError> {
    // 1. Logical Normalization (passes/logic)
    // Clean up semantics, simplify trees, and apply standard rewriting rules.
    logic::normalize(problem)?;

    // 2. Structural Normalization (passes/either-type)
    // Resolve ad-hoc types and flatten the type hierarchy into the global registry.
    typing::normalize(problem)?;

    Ok(())
}
