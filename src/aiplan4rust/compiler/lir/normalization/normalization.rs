use crate::aiplan4rust::compiler::lir::expr::iter::Scratchpad;
use crate::aiplan4rust::compiler::lir::normalization::error::NormalizationError;
use crate::aiplan4rust::compiler::lir::normalization::{logic, typing};
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;

/// Performs the complete normalization pipeline for a [`NewLiftedProblem`].
///
/// This function acts as the primary orchestrator for transforming a "raw" Lifted Intermediate
/// Representation (LIR) problem into a canonical, flat form optimized for the grounding engine.
/// To maximize CPU cache efficiency and prevent runtime allocations, it extracts the internal
/// expression storage and shares a single, pre-allocated [`Scratchpad`] scratch memory across
/// all pipeline passes.
///
/// # The Normalization Pipeline
///
/// The orchestration follows a strict deterministic sequence:
///
/// 1. **Logical Normalization ([`logic::normalize`]):**
///    - Rewrites logical implications and equivalences into base primitives (`and`, `or`, `not`).
///    - Propagates negations inward to achieve a strict Negation Normal Form (NNF).
///    - Const-folds arithmetic operations and prunes redundant boolean sub-trees.
///
/// 2. **Type Normalization ([`typing::normalize`]):**
///    - Discovers ad-hoc `Either` union type allocations nested inside expressions or parameters.
///    - Unifies and materializes these anonymous variations into permanent, atomic type definitions.
///    - Mutates all reference sites in-place to shift complex polymorphism down to $O(1)$ ID lookups.
///
/// # Errors
///
/// Returns a [`NormalizationError`] if:
/// * Expression manipulation discovers structurally malformed or broken AST links.
/// * Type discovery encounters unresolvable identifiers or registry indexing collisions.
///
/// # Examples
///
/// ```rust
/// use crate::aiplan4rust::lir::store::problem::NewLiftedProblem;
/// use crate::aiplan4rust::lir::store::normalization;
///
/// # fn run_pass(mut problem: NewLiftedProblem) -> Result<(), normalization::error::NormalizationError> {
/// // Execute logical simplification and type flattening in a single unified run
/// normalization::normalize(&mut problem)?;
/// # Ok(())
/// # }
/// ```
pub fn normalize(problem: &mut LiftedProblem) -> Result<(), NormalizationError> {
    // 1. Extract the expression old to decouple ownership and bypass Borrow Checker aliasing constraints.
    let mut store = problem.take_store();

    // 2. Allocate a single Scratchpad memory buffer to be reused throughout the entire pipeline.
    let mut scratch = Scratchpad::new();

    // 3. Execute Pass 1: Logical Normalization (NNF conversion, pruning, and const-folding).
    logic::normalize(problem, &mut store, &mut scratch)?;

    // 4. Execute Pass 2: Type Normalization (Discovery, unification, and atomic materialization).
    typing::normalize(problem, &mut store, &mut scratch)?;

    // 5. Restore the finalized and canonicalized expression old back to the problem structure.
    problem.set_store(store);

    Ok(())
}
