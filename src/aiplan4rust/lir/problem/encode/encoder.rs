//! LIR Encoding Orchestration
//!
//! This module serves as the primary entry point for transforming a PDDL domain and
//! problem from their semantic/syntactic representation into a Lifted Intermediate
//! Representation (LIR).
//!
//! The encoding process is designed to be performed in two main phases:
//! 1. **Domain Encoding**: Captures structural definitions (types, predicates, functions, and actions).
//! 2. **Problem Encoding**: Captures state-specific data (objects, initial state, and goals).
//!
//! It relies on a `LinkedSemanticContext` to resolve identifiers and maintains internal
//! mappings to ensure that references in the logic (expressions/effects) point to the
//! correct LIR indices.

use std::collections::HashMap;
use crate::aiplan4rust::arena::NodeId;
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::problem::encode::{domain_def, problem_def};

/// Extracts and encodes all domain-level elements into the LIR.
///
/// This function processes the domain AST to populate the `LiftedProblem` with
/// structural definitions. It also fills the mapping tables required for
/// symbol resolution in the problem encoding phase.
///
/// # Arguments
///
/// * `context` - The linked semantic context containing the domain's AST and symbol tables.
/// * `ir` - The mutable `LiftedProblem` where domain structures (actions, predicates) are stored.
/// * `ast_pred_to_idx` - A mutable map to be populated with the mapping: `Predicate NodeId` -> `LIR Index`.
/// * `ast_func_to_idx` - A mutable map to be populated with the mapping: `Function NodeId` -> `LIR Index`.
///
/// # Returns
///
/// * `Ok(())` - Successfully encoded the domain.
/// * `Err(LirError)` - If a structural error or semantic inconsistency is encountered.
pub fn encode_domain(
    context: &LinkedSemanticContext,
    ir: &mut LiftedProblem,
    ast_pred_to_idx: &mut HashMap<NodeId, usize>,
    ast_func_to_idx: &mut HashMap<NodeId, usize>,
) -> Result<(), LirError> {
    domain_def::encode(context, ir, ast_pred_to_idx, ast_func_to_idx)
}

/// Extracts and encodes all problem-level elements into the LIR.
///
/// This function processes the problem AST, utilizing the indices and definitions
/// collected during the domain encoding phase to resolve references in the initial
/// state and goal specifications.
///
/// # Arguments
///
/// * `context` - The linked semantic context containing the problem's AST and arena.
/// * `ir` - The mutable `LiftedProblem` where problem-specific data (objects, init, goal) is stored.
/// * `ast_pred_to_idx` - A reference to the predicate mapping populated during domain encoding.
/// * `ast_func_to_idx` - A reference to the function mapping populated during domain encoding.
///
/// # Returns
///
/// * `Ok(())` - Successfully encoded the problem.
/// * `Err(LirError)` - If an error occurs during object resolution or expression encoding.
///
/// # Errors
///
/// This function will return an error if it encounters objects or types that were
/// not defined in the domain, or if initial state expressions are malformed.
pub fn encode_problem(
    context: &LinkedSemanticContext,
    ir: &mut LiftedProblem,
    ast_pred_to_idx: &mut HashMap<NodeId, usize>,
    ast_func_to_idx: &mut HashMap<NodeId, usize>,
) -> Result<(), LirError> {
    problem_def::encode(context, ir, ast_pred_to_idx, ast_func_to_idx)
}
