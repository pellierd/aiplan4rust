//! This module defines the `LirBuilder` struct and associated functions to
//! simplify a linked and semantically verified syntax domain and problem
//! into a lifted intermediate representation (`LiftedProblem`).
//!
//! # Overview
//! The `LirBuilder` processes a `LinkedSemanticContext` containing:
//! - The domain and problem abstract syntax trees (ASTs) with all references
//!   resolved and semantic checks passed.
//! - Extracts domain elements such as types, predicates, functions, actions,
//!   and methods.
//! - Extracts problem elements such as objects, initial state, goals, and metrics.
//!
//! The resulting `LiftedProblem`:
//! - Is a structured, reusable, and symbolic representation of the syntax problem.
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
//! - This module does not perform syntax or solving itself.
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

use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::lang::{Requirement, TypedSymbol};
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::problem::{LiftedAction, LiftedMethod, InitialTaskNetwork, LiftedProblem, normalize};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::atomic_skeleton::AtomicTaskSkeleton;
use crate::aiplan4rust::lir::{LirBuilderResult, LirError};
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxSubtree};

/// This module defines the `LirBuilder`, which transforms a parsed and linked
/// syntax domain/problem into a *lifted intermediate representation* (LiftedProblem).
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
/// This module **does not** perform any syntax by itself.
/// It only prepares data for later use.
/// The input has already been verified to be semantically correct.
#[derive(Debug, Default)]
pub struct LirBuilder {
    diagnostic_manager: DiagnosticManager,
}

impl LirBuilder {

    /// Creates a new instance of IRBuilder.
    pub fn new() -> Self {
        LirBuilder {
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
    /// This function performs extraction of both the domain and problem components
    /// from the linked semantic context and builds the corresponding intermediate representation (IR).
    ///
    /// # Arguments
    ///
    /// - `context`: A mutable reference to the `LinkedSemanticContext` which contains
    ///   the parsed and linked semantic trees for domain and problem.
    ///
    /// # Returns
    ///
    /// - `Ok(LirBuilderResult)`: Contains the constructed `LiftedProblem` IR wrapped in
    ///   `Some` along with any diagnostics collected during the build process, if successful.
    /// - `Err(LirError)`: An error indicating failure during the extraction or building phase.
    ///
    /// # Errors
    ///
    /// This function propagates errors encountered during domain or problem extraction.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut builder = IRBuilder::new();
    /// let mut linked_context = /* obtain linked semantic context */;
    /// match builder.build(&mut linked_context) {
    ///     Ok(result) => {
    ///         if let Some(ir) = result.lifted_problem() {
    ///             // Use the IR here
    ///         }
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Error during IR building: {:?}", e);
    ///     }
    /// }
    /// ```
    pub fn build(
        &mut self,
        context: &mut LinkedSemanticContext,
    ) -> Result<LirBuilderResult, LirError> {
        // 1. Create a new empty LiftedProblem
        let mut lifted_problem = LiftedProblem::new();

        // 2. Populate the LiftedProblem with all domain elements
        //    such as types, constants, predicates, functions, actions, and methods
        self.extract_domain(context, &mut lifted_problem)?;

        // 3. Populate the LiftedProblem with all problem elements
        //    such as objects, initial state, goal, constraints, metric, and initial task network
        self.extract_problem(context, &mut lifted_problem)?;

        // 4. Retrieve the StringInterner from the context to handle identifiers
        let interner = context.take_interner();

        // 5. Assign the interner to the LiftedProblem
        lifted_problem.set_interner(interner);

        // 6. Normalize all expressions in the problem to canonical form
        //    This includes actions, methods, initial task network, and constraints
        normalize::normalize_problem(&mut lifted_problem)?;

        // 7. Return the successful result containing the constructed LiftedProblem
        //    and the diagnostics collected during the build process
        Ok(LirBuilderResult::success(
            lifted_problem,
            std::mem::take(&mut self.diagnostic_manager),
        ))
    }

    pub fn build_with_diagnostic_manager(
        &mut self,
        context: &mut LinkedSemanticContext,
        diagnostic_manager: DiagnosticManager
    ) -> Result<LirBuilderResult, LirError> {
        self.diagnostic_manager = diagnostic_manager;
        self.build(context)
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
    ) -> Result<(), LirError> {
        let domain_tree = context.domain_syntax_tree();

        for node in domain_tree.preorder().values() {
            let subtree = SyntaxSubtree::new(node, domain_tree);

            match subtree.node().kind() {
                AstKind::DomainName => ir.set_domain_name(subtree.node().try_ident()?),

                AstKind::RequireDef => {
                    ir.add_requirements(extract_requirements(&subtree)?);
                }
                AstKind::TypesDef => {
                    ir.add_types(extract_types(&subtree)?);
                }
                AstKind::ConstantsDef => {
                    ir.add_constants(extract_constants(&subtree)?);
                }
                AstKind::PredicatesDef => {
                    ir.add_predicates(extract_atomic_formula_skeleton(&subtree)?);
                }
                AstKind::FunctionsDef => {
                    ir.add_functions(extract_atomic_function_skeleton(&subtree)?);
                }
                AstKind::Constraints => {
                    ir.set_domain_constraints(Expr::try_from(&subtree)?);
                }
                AstKind::TaskDef => {
                    ir.add_task(AtomicTaskSkeleton::try_from(&subtree)?);
                }
                AstKind::ActionDef => {
                    ir.add_action(LiftedAction::try_from(&subtree)?);
                }
                AstKind::MethodDef => {
                    ir.add_method(LiftedMethod::try_from(&subtree)?);
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Extracts all problem-level elements (initial state, goal, metric, etc.)
    ///
    /// # Arguments
    /// - `context`: the semantic context including the problem arena.
    /// - `ir`: the LiftedProblem to fill in.
    ///
    /// # Returns
    /// - `Ok(())` if everything went fine.
    /// - `Err` otherwise.
    fn extract_problem(
        &self,
        context: &LinkedSemanticContext,
        ir: &mut LiftedProblem,
    ) -> Result<(), LirError> {
        let problem_tree = context.problem_syntax_tree();

        for node in problem_tree.preorder().values() {
            let subtree = SyntaxSubtree::new(node, problem_tree);

            match subtree.node().kind() {
                AstKind::ProblemName => ir.set_problem_name(subtree.node().try_ident()?),

                AstKind::RequireDef => {
                    ir.add_requirements(extract_requirements(&subtree)?);
                }
                AstKind::ObjectsDef => {
                    ir.add_objects(extract_constants(&subtree)?);
                }
                AstKind::Init => {
                    ir.set_init(extract_init(&subtree)?);
                }
                AstKind::Goal => {
                    ir.set_goal(extract_goal(&subtree)?);
                }
                AstKind::Constraints => {
                    ir.set_problem_constraints(Expr::try_from(&subtree)?);
                }
                AstKind::Metric => {
                    ir.set_metric_spec(Expr::try_from(&subtree)?);
                }
                AstKind::Length => {
                    ir.set_length_spec(Expr::try_from(&subtree)?);
                }
                AstKind::InitialTaskNetwork => {
                    ir.set_initial_task_network(InitialTaskNetwork::try_from(&subtree)?);
                }
                _ => {}
            }
        }

        Ok(())
    }

}

// ---------- Extraction Helpers ---------- //

/// Extracts a set of requirements from a `RequireDef` syntax subtree.
fn extract_requirements(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<HashSet<Requirement>, LirError> {
    extract_set(subtree, |child_subtree| Ok(child_subtree.node().try_requirement()?))
}

/// Extracts predicates from a `PredicatesDef` syntax subtree.
fn extract_atomic_formula_skeleton(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<HashSet<AtomicFormulaSkeleton>, LirError> {
    extract_set(subtree, |child_subtree| AtomicFormulaSkeleton::try_from(child_subtree))
}

/// Extracts functions from a `FunctionsDef` syntax subtree.
fn extract_atomic_function_skeleton(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<HashSet<AtomicFunctionSkeleton>, LirError> {
    extract_set(subtree, |child_subtree| AtomicFunctionSkeleton::try_from(child_subtree))
}

/// Extracts types from a `TypesDef` syntax subtree.
fn extract_types(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<HashSet<TypedSymbol>, LirError> {
    extract_set_from_first_child(subtree, |child_subtree| Ok(TypedSymbol::try_from(child_subtree)?))
}

/// Extracts constants or objects from a `ConstantsDef` or `ObjectsDef` syntax subtree.
fn extract_constants(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<HashSet<TypedSymbol>, LirError> {
    extract_set_from_first_child(subtree, |child_subtree| Ok(TypedSymbol::try_from(child_subtree)?))
}

/// Extracts an expression from the first child of an `Init` syntax subtree.
fn extract_init(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<Expr, LirError> {
    extract_expr_first_child(subtree)
}

/// Extracts the goal expression from a `Goal` syntax subtree.
fn extract_goal(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<Expr, LirError> {
    extract_expr_first_child(subtree)
}

/// Extracts an expression from the first child syntax subtree.
/// Used for `Init`, `Goal`, `Metric`, etc.
fn extract_expr_first_child(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<Expr, LirError> {
    let child_id = subtree.node().try_child(0)?;
    let child_node = subtree.tree().try_node(child_id)?;
    Ok(Expr::try_from(&SyntaxSubtree::new(child_node, subtree.tree()))?)
}

/// Generic helper to extract a set of elements from direct children of a syntax subtree.
/// Used for predicates, functions, requirements, etc.
fn extract_set<T, F>(
    subtree: &SyntaxSubtree<AstNode>,
    extract_fn: F,
) -> Result<HashSet<T>, LirError>
where
    T: Eq + std::hash::Hash,
    F: Fn(&SyntaxSubtree<AstNode>) -> Result<T, LirError>,
{
    let mut set = HashSet::new();
    for child_id in subtree.node().children() {
        let child_node = subtree.tree().try_node(*child_id)?;
        let child_subtree = SyntaxSubtree::new(child_node, subtree.tree());
        let value = extract_fn(&child_subtree)?;
        set.insert(value);
    }
    Ok(set)
}

/// Similar to `extract_set`, but applies the extraction function to the grandchildren
/// of the first child of the syntax subtree (used for types, constants).
fn extract_set_from_first_child<T, F>(
    subtree: &SyntaxSubtree<AstNode>,
    extract_fn: F,
) -> Result<HashSet<T>, LirError>
where
    T: Eq + std::hash::Hash,
    F: Fn(&SyntaxSubtree<AstNode>) -> Result<T, LirError>,
{
    let first_child_id = subtree.node().try_child(0)?;
    let first_child_node = subtree.tree().try_node(first_child_id)?;
    let first_child_subtree = SyntaxSubtree::new(first_child_node, subtree.tree());

    extract_set(&first_child_subtree, extract_fn)
}
