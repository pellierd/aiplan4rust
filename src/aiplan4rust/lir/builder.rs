//! This module defines the `IRBuilder` struct and associated functions to
//! transform a linked and semantically verified planning domain and problem
//! into a lifted intermediate representation (`LiftedProblem`).
//!
//! # Overview
//! The `IRBuilder` processes a `LinkedSemanticContext` containing:
//! - The domain and problem abstract syntax trees (ASTs) with all references
//!   resolved and semantic checks passed.
//! - Extracts domain elements such as types, predicates, functions, actions,
//!   and methods.
//! - Extracts problem elements such as objects, initial state, goals, and metrics.
//!
//! The resulting `LiftedProblem`:
//! - Is a structured, reusable, and symbolic representation of the planning problem.
//! - Remains "lifted", i.e., it uses symbolic references rather than grounded
//!   enumerations of instances.
//!
//! # Purpose
//! This IR is a crucial intermediate step for:
//! - Subsequent compiler passes or transformations.
//! - Planning solvers that instantiate and search for plans.
//! - Frontends for visualization or debugging.
//!
//! # Important Notes
//! - This module does not perform planning or solving itself.
//! - The input semantic context must already be validated and linked.
//!
//! # Main Components
//! - `IRBuilder`: the main struct performing extraction and building the IR.
//! - `build()`: entry point to produce the `LiftedProblem` from a semantic context.
//! - Helper functions for extracting domain and problem elements.
//!
//! # Example
//! ```ignore
//! let mut builder = IRBuilder::new();
//! let lifted_problem = builder.build(&linked_context)?;
//! ```

use std::collections::HashSet;
use std::mem::take;

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::lang::{Requirement, TypedSymbol};
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::tree::{Arena, ArenaNode};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::{LiftedAction, LiftedMethod, InitialTaskNetwork, LIRBuilderResult};
use crate::aiplan4rust::lir::LiftedProblem;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::ast::{AstKind, FromAst};
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::atomic_skeleton::AtomicTaskSkeleton;

/// This module defines the `LIRBuilder`, which transforms a parsed and linked
/// planning domain/problem into a *lifted intermediate representation* (LiftedProblem).
///
/// # What does it do?
/// - It takes as input a `LinkedSemanticContext`, which contains:
///   - The domain and problem ASTs.
///   - All references resolved (names linked to definitions).
///   - Semantic checks already passed (the input is guaranteed to be consistent).
/// - It extracts:
///   - Types, constants, predicates, functions, actions, and methods (from the domain).
///   - Objects, initial state, goals, and metrics (from the problem).
/// - It builds a structured, reusable representation of the problem.
///   - This representation is "lifted", meaning:
///     - It keeps symbolic references (e.g., variable names, types).
///     - It is not grounded yet (no enumeration of all possible substitutions).
///
/// # Why is this useful?
/// - This lifted problem can then be used by:
///   - Other compiler passes or transformations.
///   - A solver to instantiate and search for plans.
///   - A visualization frontend.
///
/// # Note
/// This module **does not** perform any planning by itself.
/// It only prepares data for later use.
/// The input has already been verified to be semantically correct.
#[derive(Debug, Default)]
pub struct LIRBuilder {
    diagnostic_manager: DiagnosticManager,
}

impl LIRBuilder {

    /// Creates a new instance of IRBuilder.
    pub fn new() -> Self {
        LIRBuilder {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }


    /// Returns an immutable reference to the internal `DiagnosticManager`,
    /// which contains diagnostics collected during the LIR building process.
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Entry point for generating a `LiftedProblem` IR from a `LinkedSemanticContext`.
    ///
    /// This function performs the extraction of both the domain and problem parts
    /// from the linked semantic context and constructs the corresponding IR representation.
    ///
    /// # Arguments
    /// - `context`: Reference to the `LinkedSemanticContext` which contains
    ///   the parsed and linked domain and problem semantic trees.
    ///
    /// # Returns
    /// - `Ok(LIRBuilderResult)` containing the constructed `LiftedProblem` IR
    ///   and accumulated diagnostics if extraction succeeds.
    /// - `Err(ParserInternalError)` if an error occurs during extraction.
    ///
    /// # Errors
    /// This function propagates errors encountered during domain or problem extraction.
    ///
    /// # Examples
    /// ```no_run
    /// let mut builder = IRBuilder::new();
    /// let result = builder.build(&linked_context)?;
    /// if let Some(ir) = result.lifted_problem() {
    ///     // Use the IR here
    /// }
    /// ```
    pub fn build(
        &mut self,
        context: &LinkedSemanticContext,
    ) -> Result<LIRBuilderResult, ParserInternalError> {
        let mut lir = LiftedProblem::new();

        self.extract_domain(context, &mut lir)?;
        self.extract_problem(context, &mut lir)?;
        Ok(LIRBuilderResult::new(Some(lir), take(&mut self.diagnostic_manager)))
    }

    /// Extracts all domain-level elements (types, predicates, actions, etc.)
    ///
    /// # Arguments
    /// - `context`: the full semantic context.
    /// - `ir`: the LiftedProblem to fill in.
    ///
    /// # Returns
    /// - A Result containing the updated IR or an error.
    pub fn extract_domain(
        &mut self,
        context: &LinkedSemanticContext,
        ir: &mut LiftedProblem,
    ) -> Result<(), ParserInternalError> {
        let domain = context.domain();

        for node in domain.preorder() {
            match node.kind() {
                AstKind::DomainName => ir.set_domain_name(node.try_ident()?),
                AstKind::RequireDef => ir.add_requirements(extract_requirements(node, domain)?),
                AstKind::TypesDef => ir.add_types(extract_types(node, domain)?),
                AstKind::ConstantsDef => ir.add_constants(extract_constants(node, domain)?),
                AstKind::PredicatesDef => ir.add_predicates(extract_atomic_formula_skeleton(node, domain)?),
                AstKind::FunctionsDef => ir.add_functions(extract_atomic_function_skeleton(node, domain)?),
                AstKind::Constraints => {
                    ir.set_domain_constraints(Expr::from_ast(node, domain)?)
                }
                AstKind::TaskDef => ir.add_task(AtomicTaskSkeleton::from_ast(node, domain)?),
                AstKind::ActionDef => ir.add_action(LiftedAction::from_ast(node, domain)?),
                AstKind::MethodDef => ir.add_method(LiftedMethod::from_ast(node, domain)?),
                _ => {}
            }
        }

        Ok(())
    }

    /// Extracts all problem-level elements (initial state, goal, metric, etc.)
    ///
    /// # Arguments
    /// - `context`: the semantic context including the problem tree.
    /// - `ir`: the LiftedProblem to fill in.
    ///
    /// # Returns
    /// - `Ok(())` if everything went fine.
    /// - `Err` otherwise.
    fn extract_problem(
        &self,
        context: &LinkedSemanticContext,
        ir: &mut LiftedProblem,
    ) -> Result<(), ParserInternalError> {
        let problem = context.problem();

        for node in problem.preorder() {
            match node.kind() {
                AstKind::ProblemName => ir.set_problem_name(node.try_ident()?),
                AstKind::RequireDef => ir.add_requirements(extract_requirements(node, problem)?),
                AstKind::ObjectsDef => ir.add_objects(extract_constants(node, problem)?),
                AstKind::Init => ir.set_init(extract_init(node, problem)?),
                AstKind::Goal => ir.set_goal(extract_goal(node, problem)?),
                AstKind::Constraints => {
                    ir.set_problem_constraints(Expr::from_ast(node, problem)?)
                }
                AstKind::Metric => ir.set_metric_spec(Expr::from_ast(node, problem)?),
                AstKind::Length => ir.set_length_spec(Expr::from_ast(node, problem)?),
                AstKind::InitialTaskNetwork => {
                    ir.set_initial_task_network(InitialTaskNetwork::from_ast(node, problem)?)
                }
                _ => {}
            }
        }

        Ok(())
    }
}

// ---------- Extraction Helpers ---------- //

/// Extracts a set of requirements from a `RequireDef` node.
fn extract_requirements(
    node: &AstNode,
    ast: &Arena<AstNode>,
) -> Result<HashSet<Requirement>, ParserInternalError> {
    extract_set(node, ast, |n, _| n.try_requirement())
}

/// Extracts predicates from a `PredicatesDef` node.
fn extract_atomic_formula_skeleton(
    node: &AstNode,
    ast: &Arena<AstNode>,
) -> Result<HashSet<AtomicFormulaSkeleton>, ParserInternalError> {
    extract_set(node, ast, AtomicFormulaSkeleton::from_ast)
}

/// Extracts functions from a `FunctionsDef` node.
fn extract_atomic_function_skeleton(
    node: &AstNode,
    ast: &Arena<AstNode>,
) -> Result<HashSet<AtomicFunctionSkeleton>, ParserInternalError> {
    extract_set(node, ast, AtomicFunctionSkeleton::from_ast)
}

/// Extracts types from a `TypesDef` node.
fn extract_types(
    node: &AstNode,
    ast: &Arena<AstNode>,
) -> Result<HashSet<TypedSymbol>, ParserInternalError> {
    extract_set_from_first_child(node, ast, TypedSymbol::from_ast)
}

/// Extracts constants or objects from a `ConstantsDef` or `ObjectsDef` node.
fn extract_constants(
    node: &AstNode,
    ast: &Arena<AstNode>,
) -> Result<HashSet<TypedSymbol>, ParserInternalError> {
    extract_set_from_first_child(node, ast, TypedSymbol::from_ast)
}

/// Extracts an expression from the first child of an `Init` node.
fn extract_init(
    node: &AstNode,
    ast: &Arena<AstNode>,
) -> Result<Expr, ParserInternalError> {
    extract_expr_first_child(node, ast)
}

/// Extracts the goal expression from a `Goal` node.
fn extract_goal(
    node: &AstNode,
    ast: &Arena<AstNode>,
) -> Result<Expr, ParserInternalError> {
    extract_expr_first_child(node, ast)
}

/// Extracts an expression from the first child node.
/// Used for `Init`, `Goal`, `Metric`, etc.
fn extract_expr_first_child(
    node: &AstNode,
    ast: &Arena<AstNode>,
) -> Result<Expr, ParserInternalError> {
    let child_id = node.try_child(0)?;
    let child_node = ast.try_node(child_id)?;
    Expr::from_ast(child_node, ast)
}

/// Generic helper to extract a set of elements from direct children of a node.
/// Used for predicates, functions, requirements, etc.
fn extract_set<T, F>(
    node: &AstNode,
    ast: &Arena<AstNode>,
    extract_fn: F,
) -> Result<HashSet<T>, ParserInternalError>
where
    T: Eq + std::hash::Hash,
    F: Fn(&AstNode, &Arena<AstNode>) -> Result<T, ParserInternalError>,
{
    let mut set = HashSet::new();
    for child_id in node.children() {
        let child_node = ast.try_node(*child_id)?;
        let value = extract_fn(child_node, ast)?;
        set.insert(value);
    }
    Ok(set)
}

/// Similar to `extract_set`, but applies the extraction function to the grandchildren
/// of the first child of the node (used for types, constants).
fn extract_set_from_first_child<T, F>(
    node: &AstNode,
    ast: &Arena<AstNode>,
    extract_fn: F,
) -> Result<HashSet<T>, ParserInternalError>
where
    T: Eq + std::hash::Hash,
    F: Fn(&AstNode, &Arena<AstNode>) -> Result<T, ParserInternalError>,
{
    let first_child_id = node.try_child(0)?;
    let first_child_node = ast.try_node(first_child_id)?;

    extract_set(first_child_node, ast, extract_fn)
}
