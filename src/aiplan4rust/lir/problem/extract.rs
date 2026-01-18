use std::collections::HashSet;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{Requirement, TypedSymbol};
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::lir::atomic_skeleton::{AtomicFormulaSkeleton, AtomicFunctionSkeleton, AtomicTaskSkeleton};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::{InitialTaskNetwork, LiftedAction, LiftedDerivedPredicate, LiftedDurativeAction, LiftedMethod, LiftedProblem};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxSubtree};

/// Extracts all domain-level elements (types, predicates, actions, etc.)
///
/// # Arguments
/// - `context`: the full semantic context.
/// - `ir`: the LiftedProblem to fill in.
///
/// # Returns
/// - A Result containing the updated IR or an error.
pub(crate) fn extract_domain(
    context: &LinkedSemanticContext,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let domain_tree = context.domain_syntax_tree();

    for node in domain_tree.preorder().values() {
        let subtree = SyntaxSubtree::new(node, domain_tree);

        match subtree.node().kind() {
            AstKind::DomainName => {
                let id = subtree.node().try_ident()?;
                ir.set_domain_id(id)?;
            }
            AstKind::TypesDef => {
                let types = extract_types(&subtree)?;
                ir.add_types(types);
            }
            AstKind::ConstantsDef => {
                let constants = extract_constants(&subtree)?;
                ir.add_constants(constants);
            }
            AstKind::PredicatesDef => {
                let predicates = extract_atomic_formula_skeleton(&subtree)?;
                ir.add_predicates(predicates);
            }
            AstKind::FunctionsDef => {
                let functions = extract_atomic_function_skeleton(&subtree)?;
                ir.add_functions(functions);
            }
            AstKind::Constraints => {
                let constraints = Expr::try_from(&subtree)?;
                ir.set_domain_constraints(constraints);
            }
            AstKind::TaskDef => {
                let task = AtomicTaskSkeleton::try_from(&subtree)?;
                ir.add_task(task);
            }
            AstKind::ActionDef => {
                let action = LiftedAction::try_from(&subtree)?;
                ir.add_action(action);
            }
            AstKind::DurativeActionDef => {
                let action = LiftedDurativeAction::try_from(&subtree)?;
                ir.add_durative_action(action);
            }
            AstKind::DerivedDef => {
                let derived_predicate = LiftedDerivedPredicate::try_from(&subtree)?;
                ir.add_derived_predicate(derived_predicate);
            }
            AstKind::MethodDef => {
                let method = LiftedMethod::try_from(&subtree)?;
                ir.add_method(method);
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
pub(crate) fn extract_problem(
    context: &LinkedSemanticContext,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let problem_tree = context.problem_syntax_tree();

    for node in problem_tree.preorder().values() {
        let subtree = SyntaxSubtree::new(node, problem_tree);

        match subtree.node().kind() {
            AstKind::ProblemName => {
                let id = subtree.node().try_ident()?;
                ir.set_problem_id(id)?;
            }
            AstKind::ObjectsDef => {
                let objects = extract_constants(&subtree)?;
                ir.add_objects(objects);
            }
            AstKind::Init => {
                let init_expr = extract_init(&subtree)?;
                ir.set_init(init_expr);
            }
            AstKind::Goal => {
                let goal_expr = extract_goal(&subtree)?;
                ir.set_goal(goal_expr);
            }
            AstKind::Constraints => {
                let constraints = Expr::try_from(&subtree)?;
                ir.set_problem_constraints(constraints);
            }
            AstKind::Metric => {
                let metric = Expr::try_from(&subtree)?;
                ir.set_metric_spec(metric);
            }
            AstKind::Length => {
                let length = Expr::try_from(&subtree)?;
                ir.set_length_spec(length);
            }
            AstKind::InitialTaskNetwork => {
                let network = InitialTaskNetwork::try_from(&subtree)?;
                ir.set_initial_task_network(network);
            }
            _ => {}
        }
    }

    Ok(())
}

// ---------- Extraction Helpers ---------- //

/// Extracts a set of requirements from a `RequireDef` syntax subtree.
#[allow(dead_code)]
fn extract_requirements(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<HashSet<Requirement>, LirError> {
    extract_set(subtree, |child_subtree| {
        Ok(child_subtree.node().try_requirement()?)
    })
}

/// Extracts predicates from a `PredicatesDef` syntax subtree.
fn extract_atomic_formula_skeleton(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<HashSet<AtomicFormulaSkeleton>, LirError> {
    extract_set(subtree, |child_subtree| {
        AtomicFormulaSkeleton::try_from(child_subtree)
    })
}

/// Extracts functions from a `FunctionsDef` syntax subtree.
fn extract_atomic_function_skeleton(
    subtree: &SyntaxSubtree<AstNode>,
) -> Result<HashSet<AtomicFunctionSkeleton>, LirError> {
    extract_set(subtree, |child_subtree| {
        AtomicFunctionSkeleton::try_from(child_subtree)
    })
}

/// Extracts types from a `TypesDef` syntax subtree into a map from Ident to TypedSymbol.
fn extract_types(subtree: &SyntaxSubtree<AstNode>) -> Result<HashSet<TypedSymbol>, LirError> {
    // Apply extraction function to the grandchildren of the first child
    extract_set_from_first_child(subtree, |child_subtree| {
        Ok(TypedSymbol::try_from(child_subtree)?)
    })
}


/// Extracts constants or objects from a `ConstantsDef` or `ObjectsDef` syntax subtree.
fn extract_constants(subtree: &SyntaxSubtree<AstNode>) -> Result<HashSet<TypedSymbol>, LirError> {
    extract_set_from_first_child(subtree, |child_subtree| {
        Ok(TypedSymbol::try_from(child_subtree)?)
    })
}

/// Extracts an expression from the first child of an `Init` syntax subtree.
fn extract_init(subtree: &SyntaxSubtree<AstNode>) -> Result<Expr, LirError> {
    extract_expr_first_child(subtree)
}

/// Extracts the goal expression from a `Goal` syntax subtree.
fn extract_goal(subtree: &SyntaxSubtree<AstNode>) -> Result<Expr, LirError> {
    extract_expr_first_child(subtree)
}

/// Extracts an expression from the first child syntax subtree.
/// Used for `Init`, `Goal`, `Metric`, etc.
fn extract_expr_first_child(subtree: &SyntaxSubtree<AstNode>) -> Result<Expr, LirError> {
    let child_id = subtree.node().try_child(0)?;
    let child_node = subtree.tree().try_node(child_id)?;
    Ok(Expr::try_from(&SyntaxSubtree::new(
        child_node,
        subtree.tree(),
    ))?)
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
